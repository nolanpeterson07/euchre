use log::{info, warn};
use rand::seq::SliceRandom;
use shared::{
    Bidder, Card, ClientMessage, Game, MAX_PLAYERS, Phase, PlayedCard, Rank, ServerMessage, Suit,
    Team,
};

const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

const RANKS: [Rank; 6] = [
    Rank::Nine,
    Rank::Ten,
    Rank::Jack,
    Rank::Queen,
    Rank::King,
    Rank::Ace,
];

const WINNING_SCORE: u8 = 10;

const TRICKS_PER_HAND: u8 = 5;

pub fn apply(
    game: &mut Game,
    players: &[String],
    player: &str,
    action: &ClientMessage,
) -> Result<ServerMessage, String> {
    match action {
        ClientMessage::Chat { text } => Ok(ServerMessage::Chat {
            from: player.to_string(),
            text: text.clone(),
        }),
        ClientMessage::StartGame => {
            if game.phase != Phase::Lobby {
                return Err("game already started".into());
            }
            if players.len() != MAX_PLAYERS {
                return Err(format!("need {MAX_PLAYERS} players"));
            }
            game.teams = [
                Team {
                    players: [0, 2],
                    ..Team::default()
                },
                Team {
                    players: [1, 3],
                    ..Team::default()
                },
            ];
            deal(game);
            info!("game started, dealer={}", game.dealer);
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::OrderUp { alone } => {
            require_phase(game, Phase::Bidding1)?;
            require_turn(game, players, player)?;

            let upcard = game.upcard.take().ok_or("no upcard")?;
            game.trump = Some(upcard.suit);
            game.maker = Some(Bidder {
                player: game.turn,
                alone: *alone,
            });
            if sitting_out(game) == Some(game.dealer) {
                game.phase = Phase::Playing;
                game.turn = next_seat(game, game.dealer);
            } else {
                game.hands[game.dealer].push(upcard);
                game.phase = Phase::AwaitingDiscard;
                game.turn = game.dealer;
            }

            info!("{player} ordered up, alone={alone}");
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::CallTrump { suit, alone } => {
            require_phase(game, Phase::Bidding2)?;
            require_turn(game, players, player)?;
            if game.upcard.is_some_and(|c| c.suit == *suit) {
                return Err("can't call the turned-down suit".into());
            }

            game.trump = Some(*suit);
            game.maker = Some(Bidder {
                player: game.turn,
                alone: *alone,
            });
            game.phase = Phase::Playing;
            game.turn = next_seat(game, game.dealer); // dealer's left leads the first trick

            info!("{player} called {suit:?}, alone={alone}");
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::Pass => {
            if !matches!(game.phase, Phase::Bidding1 | Phase::Bidding2) {
                warn!("{player} tried to pass outside of bidding");
                return Err("not bidding right now".into());
            }
            let turn = require_turn(game, players, player)?;

            if turn == game.dealer {
                // dealer is always last to act in a bidding round
                match game.phase {
                    Phase::Bidding1 => {
                        game.phase = Phase::Bidding2;
                        game.turn = left_of_dealer(game, players);
                    }
                    Phase::Bidding2 => {
                        return Err("dealer must call trump".into());
                    }
                    _ => unreachable!(),
                }
            } else {
                game.turn = (turn + 1) % players.len();
            }
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::Discard { card } => {
            require_phase(game, Phase::AwaitingDiscard)?;
            let turn = require_turn(game, players, player)?;

            if card == &game.upcard.unwrap() {
                return Err("cannot discard the upcard".into());
            }

            take_card(game, turn, card)?;
            game.phase = Phase::Playing;
            game.turn = next_seat(game, game.dealer);

            info!("{player} discarded");
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::PlayCard { card } => {
            require_phase(game, Phase::Playing)?;
            let turn = require_turn(game, players, player)?;
            let trump = game.trump.ok_or("no trump set")?;

            if !game.hands[turn].contains(card) {
                return Err("you don't have that card".into());
            }
            if let Some(lead) = game.trick.first() {
                let led = effective_suit(lead.card, trump);
                if effective_suit(*card, trump) != led
                    && game.hands[turn]
                        .iter()
                        .any(|c| effective_suit(*c, trump) == led)
                {
                    return Err("must follow suit".into());
                }
            }
            take_card(game, turn, card)?;
            game.trick.push(PlayedCard {
                player: turn,
                card: *card,
            });
            info!("{player} played {card:?}");

            let trick_size = if game.maker.is_some_and(|m| m.alone) {
                3
            } else {
                4
            };
            if game.trick.len() < trick_size {
                game.turn = next_seat(game, turn);
            } else {
                let led = effective_suit(game.trick[0].card, trump);
                let winner = game
                    .trick
                    .iter()
                    .max_by_key(|p| power(p.card, led, trump))
                    .unwrap()
                    .player;

                game.trick.clear();
                game.teams[winner % 2].tricks_won += 1;
                game.turn = winner;
                info!("trick won by seat {winner}");

                if game.teams[0].tricks_won + game.teams[1].tricks_won == TRICKS_PER_HAND {
                    score_hand(game, players);
                }
            }
            Ok(ServerMessage::GameState { game: game.clone() })
        }
    }
}

fn deal(game: &mut Game) {
    let mut deck: Vec<Card> = SUITS
        .iter()
        .flat_map(|&suit| RANKS.iter().map(move |&rank| Card { rank, suit }))
        .collect();
    deck.shuffle(&mut rand::rng());
    for hand in &mut game.hands {
        *hand = deck.drain(..5).collect();
    }
    game.upcard = deck.pop();
    game.trump = None;
    game.maker = None;
    game.trick.clear();
    for team in &mut game.teams {
        team.tricks_won = 0;
    }
    game.phase = Phase::Bidding1;
    game.turn = (game.dealer + 1) % MAX_PLAYERS;
}

/// Award points for the finished hand, then either end the game or deal the next hand.
fn score_hand(game: &mut Game, players: &[String]) {
    let maker = game.maker.expect("hand scored without a maker");
    let maker_team = maker.player % 2;
    let tricks = game.teams[maker_team].tricks_won;
    let (winner, points) = if tricks >= 3 {
        let points = match (tricks, maker.alone) {
            (5, true) => 4,
            (5, false) => 2,
            _ => 1,
        };
        (maker_team, points)
    } else {
        (1 - maker_team, 2) // euchred
    };
    game.teams[winner].score += points;
    info!("hand over: team {winner} scores {points}");

    if game.teams[winner].score >= WINNING_SCORE {
        game.phase = Phase::GameOver;
    } else {
        game.dealer = (game.dealer + 1) % players.len();
        deal(game);
    }
}

/// The seat skipped when the maker plays alone (their partner), if any.
fn sitting_out(game: &Game) -> Option<usize> {
    game.maker
        .filter(|m| m.alone)
        .map(|m| (m.player + 2) % MAX_PLAYERS)
}

/// Next seat clockwise from `from`, skipping a sitting-out partner.
fn next_seat(game: &Game, from: usize) -> usize {
    let mut seat = (from + 1) % MAX_PLAYERS;
    if Some(seat) == sitting_out(game) {
        seat = (seat + 1) % MAX_PLAYERS;
    }
    seat
}

/// The suit a card counts as: the left bower plays as trump.
fn effective_suit(card: Card, trump: Suit) -> Suit {
    if card.rank == Rank::Jack && is_red(card.suit) == is_red(trump) {
        trump
    } else {
        card.suit
    }
}

fn is_red(suit: Suit) -> bool {
    matches!(suit, Suit::Diamonds | Suit::Hearts)
}

/// Strength of a card in a trick: bowers > trump > led suit > everything else (0).
fn power(card: Card, led: Suit, trump: Suit) -> u8 {
    let rank = card.rank as u8; // Nine=0 .. Ace=5
    if effective_suit(card, trump) == trump {
        match (card.rank, card.suit == trump) {
            (Rank::Jack, true) => 20,  // right bower
            (Rank::Jack, false) => 19, // left bower
            _ => 12 + rank,
        }
    } else if card.suit == led {
        1 + rank
    } else {
        0
    }
}

fn left_of_dealer(game: &Game, players: &[String]) -> usize {
    (game.dealer + 1) % players.len()
}

fn require_phase(game: &Game, expected: Phase) -> Result<(), String> {
    if game.phase != expected {
        return Err("not the right time for that".into());
    }
    Ok(())
}

/// Confirms `player` is the one `game.turn` is currently waiting on.
fn require_turn(game: &Game, players: &[String], player: &str) -> Result<usize, String> {
    if players.get(game.turn).map(String::as_str) != Some(player) {
        return Err("not your turn".into());
    }
    Ok(game.turn)
}

/// Removes `card` from `seat`'s hand, or errors if they don't hold it.
fn take_card(game: &mut Game, seat: usize, card: &Card) -> Result<(), String> {
    let pos = game.hands[seat]
        .iter()
        .position(|c| c == card)
        .ok_or("you don't have that card")?;
    game.hands[seat].remove(pos);
    Ok(())
}
