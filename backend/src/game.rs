use log::{info, warn};
use shared::{ClientMessage, Game, MAX_PLAYERS, Phase, ServerMessage};

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
            game.phase = Phase::Bidding1;
            game.turn = left_of_dealer(game, players);
            info!("game started, dealer={}", game.dealer);
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::OrderUp { alone } => {
            require_phase(game, Phase::Bidding1)?;
            require_turn(game, players, player)?;

            game.trump = game.upcard.map(|c| c.suit);
            game.maker = Some(shared::Bidder {
                player: game.turn,
                alone: *alone,
            });
            game.phase = Phase::AwaitingDiscard;
            game.turn = game.dealer; // dealer must discard the picked-up card

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
            game.maker = Some(shared::Bidder {
                player: game.turn,
                alone: *alone,
            });
            game.phase = Phase::Playing;
            game.turn = left_of_dealer(game, players); // dealer's left leads the first trick

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
                        return Err("all players passed; redeal not implemented".into());
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

            discard_if_held(game, turn, card);
            game.phase = Phase::Playing;
            game.turn = left_of_dealer(game, players); // dealer's left leads the first trick

            info!("{player} discarded");
            Ok(ServerMessage::GameState { game: game.clone() })
        }
        ClientMessage::PlayCard { card } => {
            require_phase(game, Phase::Playing)?;
            let turn = require_turn(game, players, player)?;

            discard_if_held(game, turn, card);
            game.trick.push(shared::PlayedCard {
                player: turn,
                card: *card,
            });
            // TODO: once trick.len() == 4, score the trick, clear it, and
            // set turn to the winner instead of just the next seat.
            game.turn = (turn + 1) % players.len();

            info!("{player} played {card:?}");
            Ok(ServerMessage::GameState { game: game.clone() })
        }
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

/// Removes `card` from `seat`'s hand, if dealing has put it there.
fn discard_if_held(game: &mut Game, seat: usize, card: &shared::Card) {
    if let Some(pos) = game.hands[seat].iter().position(|c| c == card) {
        game.hands[seat].remove(pos);
    }
}
