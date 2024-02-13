use curvefever_derive::EnumTryFromRepr;

#[derive(Debug)]
pub enum ClientEvent {
    SyncPlayers,
    Input { player_id: u16, dir: Direction },
    AddPlayer { request_id: u64 },
    Rename { player_id: u16, name: String },
    PrevColor { player_id: u16 },
    NextColor { player_id: u16 },
    Restart,
    Pause,
    Share,
    Help,
}

impl ClientEvent {
    pub const TYPE_SYNC_PLAYERS: u8 = 1;
    pub const TYPE_INPUT: u8 = 2;
    pub const TYPE_ADD_PLAYER: u8 = 3;
    pub const TYPE_RENAME: u8 = 4;
    pub const TYPE_PREV_COLOR: u8 = 5;
    pub const TYPE_NEXT_COLOR: u8 = 6;
    pub const TYPE_RESTART: u8 = 7;
    pub const TYPE_PAUSE: u8 = 8;
    pub const TYPE_SHARE: u8 = 9;
    pub const TYPE_HELP: u8 = 10;

    pub fn encode(&self, stream: &mut impl std::io::Write) -> anyhow::Result<()> {
        match self {
            ClientEvent::SyncPlayers => {
                stream.write_all(&[Self::TYPE_SYNC_PLAYERS])?;
            }
            ClientEvent::Input { player_id, dir } => {
                stream.write_all(&[Self::TYPE_INPUT])?;
                stream.write_all(&u16::to_le_bytes(*player_id))?;
                stream.write_all(&[*dir as u8])?;
            }
            ClientEvent::AddPlayer { request_id } => {
                stream.write_all(&[Self::TYPE_ADD_PLAYER])?;
                stream.write_all(&u64::to_le_bytes(*request_id))?;
            }
            ClientEvent::Rename { player_id, name } => {
                stream.write_all(&[Self::TYPE_RENAME])?;
                stream.write_all(&u16::to_le_bytes(*player_id))?;
                write_string(stream, name)?;
            }
            ClientEvent::PrevColor { player_id } => {
                stream.write_all(&[Self::TYPE_PREV_COLOR])?;
                stream.write_all(&u16::to_le_bytes(*player_id))?;
            }
            ClientEvent::NextColor { player_id } => {
                stream.write_all(&[Self::TYPE_NEXT_COLOR])?;
                stream.write_all(&u16::to_le_bytes(*player_id))?;
            }
            ClientEvent::Restart => {
                stream.write_all(&[Self::TYPE_RESTART])?;
            }
            ClientEvent::Pause => {
                stream.write_all(&[Self::TYPE_PAUSE])?;
            }
            ClientEvent::Share => {
                stream.write_all(&[Self::TYPE_SHARE])?;
            }
            ClientEvent::Help => {
                stream.write_all(&[Self::TYPE_HELP])?;
            }
        }

        Ok(())
    }

    pub fn decode(stream: &mut impl std::io::Read) -> anyhow::Result<Self> {
        let ty = read_u8(stream)?;
        let event = match ty {
            Self::TYPE_SYNC_PLAYERS => ClientEvent::SyncPlayers,
            Self::TYPE_INPUT => {
                let player_id = read_u16(stream)?;
                let dir = read_u8(stream)?;
                let Ok(dir) = Direction::try_from(dir) else {
                    anyhow::bail!("unknown direction {}", dir);
                };

                ClientEvent::Input { player_id, dir }
            }
            Self::TYPE_ADD_PLAYER => {
                let request_id = read_u64(stream)?;
                ClientEvent::AddPlayer { request_id }
            }
            Self::TYPE_RENAME => {
                let player_id = read_u16(stream)?;
                let name = read_string(stream)?;
                ClientEvent::Rename { player_id, name }
            }
            Self::TYPE_PREV_COLOR => {
                let player_id = read_u16(stream)?;
                ClientEvent::PrevColor { player_id }
            }
            Self::TYPE_NEXT_COLOR => {
                let player_id = read_u16(stream)?;
                ClientEvent::NextColor { player_id }
            }
            Self::TYPE_RESTART => ClientEvent::Restart,
            Self::TYPE_PAUSE => ClientEvent::Pause,
            Self::TYPE_SHARE => ClientEvent::Share,
            Self::TYPE_HELP => ClientEvent::Help,
            _ => {
                anyhow::bail!("Unknown ClientEvent type: {}", ty);
            }
        };

        Ok(event)
    }
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    Exit,
    PlayerSync {
        players: Vec<Player>,
    },
    /// Response to a [`ClientEvent::AddPlayer`].
    PlayerAdded {
        request_id: u64,
        player: Player,
    },
}

impl GameEvent {
    pub const TYPE_EXIT: u8 = 1;
    pub const TYPE_PLAYER_LIST: u8 = 2;
    pub const TYPE_PLAYER_ADDED: u8 = 3;

    pub fn encode(&self, stream: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            GameEvent::Exit => {
                stream.write_all(&[Self::TYPE_EXIT])?;
            }
            GameEvent::PlayerSync { players } => {
                stream.write_all(&[Self::TYPE_PLAYER_LIST])?;
                stream.write_all(&u16::to_le_bytes(players.len() as u16))?;
                for p in players.iter() {
                    p.encode(stream)?;
                }
            }
            GameEvent::PlayerAdded { request_id, player } => {
                stream.write_all(&[Self::TYPE_PLAYER_ADDED])?;
                stream.write_all(&u64::to_le_bytes(*request_id))?;
                player.encode(stream)?;
            }
        }

        Ok(())
    }

    pub fn decode(stream: &mut impl std::io::Read) -> anyhow::Result<Self> {
        let ty = read_u8(stream)?;

        let event = match ty {
            Self::TYPE_EXIT => GameEvent::Exit,
            Self::TYPE_PLAYER_LIST => {
                let num_players = read_u16(stream)?;
                let mut players = Vec::with_capacity(num_players as usize);
                for _ in 0..num_players {
                    players.push(Player::decode(stream)?);
                }
                GameEvent::PlayerSync { players }
            }
            Self::TYPE_PLAYER_ADDED => {
                let request_id = read_u64(stream)?;
                let player = Player::decode(stream)?;
                GameEvent::PlayerAdded { request_id, player }
            }
            _ => {
                anyhow::bail!("Unknown GameEvent type: {}", ty);
            }
        };

        Ok(event)
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: u16,
    pub color: [u8; 4],
    pub name: String,
}

impl Player {
    pub fn new(id: u16, color: [u8; 4], name: String) -> Self {
        Self { id, color, name }
    }

    pub fn encode(&self, stream: &mut impl std::io::Write) -> std::io::Result<()> {
        stream.write_all(&self.color)?;
        stream.write_all(&u16::to_le_bytes(self.id))?;
        write_string(stream, &self.name)?;
        Ok(())
    }

    pub fn decode(stream: &mut impl std::io::Read) -> std::io::Result<Player> {
        let mut color = [0; 4];
        stream.read_exact(&mut color)?;
        let id = read_u16(stream)?;
        let name = read_string(stream)?;
        Ok(Player { id, color, name })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumTryFromRepr)]
#[cods(repr = u8)]
pub enum Direction {
    Straight = 0,
    Right = 1,
    Left = 2,
}

impl Direction {
    pub fn from_left_right_down(left_down: bool, right_down: bool) -> Self {
        match (left_down, right_down) {
            (true, true) | (false, false) => Self::Straight,
            (true, false) => Self::Left,
            (false, true) => Self::Right,
        }
    }
}

fn read_u8(stream: &mut impl std::io::Read) -> std::io::Result<u8> {
    let mut buf = [0];
    stream.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16(stream: &mut impl std::io::Read) -> std::io::Result<u16> {
    let mut buf = [0; 2];
    stream.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u64(stream: &mut impl std::io::Read) -> std::io::Result<u64> {
    let mut buf = [0; 8];
    stream.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_string(stream: &mut impl std::io::Write, name: &str) -> std::io::Result<()> {
    stream.write_all(&u16::to_le_bytes(name.len() as u16))?;
    stream.write_all(name.as_bytes())?;
    Ok(())
}

fn read_string(stream: &mut impl std::io::Read) -> std::io::Result<String> {
    let name_len = read_u16(stream)?;
    let mut name_buf = vec![0; name_len as usize];
    stream.read_exact(&mut name_buf)?;
    let name = String::from_utf8(name_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(name)
}
