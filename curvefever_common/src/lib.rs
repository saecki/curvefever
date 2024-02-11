use curvefever_derive::EnumTryFromRepr;

#[derive(Debug)]
pub enum ClientEvent {
    Input { player_id: u16, dir: Direction },
    ListPlayers,
}

impl ClientEvent {
    pub const TYPE_INPUT: u8 = 0x01;
    pub const TYPE_LIST_PLAYERS: u8 = 0x02;

    pub fn encode(&self, stream: &mut impl std::io::Write) -> anyhow::Result<()> {
        match self {
            ClientEvent::Input { player_id, dir } => {
                stream.write_all(&[Self::TYPE_INPUT])?;
                stream.write_all(&u16::to_le_bytes(*player_id))?;
                stream.write_all(&[*dir as u8])?;
            }
            ClientEvent::ListPlayers => {
                stream.write_all(&[Self::TYPE_LIST_PLAYERS])?;
            }
        }

        Ok(())
    }

    pub fn decode(stream: &mut impl std::io::Read) -> anyhow::Result<Self> {
        let ty = read_u8(stream)?;
        let event = match ty {
            Self::TYPE_INPUT => {
                let player_id = read_u16(stream)?;
                let dir = read_u8(stream)?;
                let Ok(dir) = Direction::try_from(dir) else {
                    anyhow::bail!("unknown direction {}", dir);
                };

                ClientEvent::Input { player_id, dir }
            }
            Self::TYPE_LIST_PLAYERS => ClientEvent::ListPlayers,
            _ => {
                anyhow::bail!("Unknown ClientEvent type: {}", ty);
            }
        };

        Ok(event)
    }
}

#[derive(Debug)]
pub enum GameEvent {
    PlayerList(Vec<Player>),
    Exit,
}

impl GameEvent {
    pub const TYPE_EXIT: u8 = 0x01;
    pub const TYPE_PLAYER_LIST: u8 = 0x02;

    pub fn encode(&self, stream: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            GameEvent::PlayerList(players) => {
                stream.write_all(&[Self::TYPE_PLAYER_LIST])?;
                stream.write_all(&u16::to_le_bytes(players.len() as u16))?;
                for p in players.iter() {
                    p.encode(stream)?;
                }
            }
            GameEvent::Exit => {
                stream.write_all(&[Self::TYPE_EXIT])?;
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
                GameEvent::PlayerList(players)
            }
            _ => {
                anyhow::bail!("Unknown GameEvent type: {}", ty);
            }
        };

        Ok(event)
    }
}

#[derive(Debug)]
pub struct Player {
    id: u16,
    name: String,
}

impl Player {
    pub fn encode(&self, stream: &mut impl std::io::Write) -> std::io::Result<()> {
        stream.write_all(&u16::to_le_bytes(self.id))?;

        stream.write_all(&u16::to_le_bytes(self.name.len() as u16))?;
        stream.write_all(&self.name.as_bytes())?;

        Ok(())
    }

    pub fn decode(stream: &mut impl std::io::Read) -> std::io::Result<Player> {
        let id = read_u16(stream)?;

        let name_len = read_u16(stream)?;

        let mut name_buf = vec![0; name_len as usize];
        stream.read_exact(&mut name_buf)?;
        let name = match String::from_utf8(name_buf) {
            Ok(s) => s,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
        };

        Ok(Player { id, name })
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
