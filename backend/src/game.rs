use shared::{ClientMessage, Game, MAX_PLAYERS, ServerMessage};

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
            if game.started {
                return Err("game already started".into());
            }
            if players.len() != MAX_PLAYERS {
                return Err(format!("need {MAX_PLAYERS} players"));
            }
            game.started = true;
            Ok(ServerMessage::GameState { game: game.clone() })
        }
    }
}
