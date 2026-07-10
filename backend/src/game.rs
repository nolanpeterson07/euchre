use shared::{ClientMessage, Game, MAX_PLAYERS, ServerMessage};

pub fn apply(
    game: &mut Game,
    players: &[String],
    player: &str,
    action: &ClientMessage,
) -> Result<ServerMessage, String> {
    let _ = player;
    match action {
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
        
        ClientMessage::Chat { .. } => Err("not a game action".into()),
    }
}
