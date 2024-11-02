use crate::Package;

pub enum Response {
    Ack,
    Err(String),
    Info(Vec<String>),
    Msg(String, String, String),
}

impl Response {
    pub fn err(why: impl Into<String>) -> Self {
        Self::Err(why.into())
    }

    pub fn info(data: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Info(data.into_iter().map(Into::into).collect())
    }

    pub fn is_bad(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn into_package(self) -> Package {
        self.into()
    }
}

impl From<Response> for Package {
    fn from(value: Response) -> Self {
        match value {
            Response::Ack => Self {
                cmd: "ack".to_string(),
                args: Vec::new(),
            },
            Response::Err(why) => Self {
                cmd: "err".to_string(),
                args: vec![why],
            },
            Response::Info(data) => Self {
                cmd: "info".to_string(),
                args: data,
            },
            Response::Msg(channel, name, msg) => Self {
                cmd: "msg".to_string(),
                args: vec![channel, name, msg],
            }
        }
    }
}

#[derive(Debug)]
pub enum ResponseError {
    UnknownCmd(String),
    MissingArgs(&'static str),
}

impl TryFrom<Package> for Response {
    type Error = ResponseError;
    
    fn try_from(value: Package) -> Result<Self, Self::Error> {
        Ok(match value.cmd.as_str() {
            "ack" => Self::Ack,
            "err" => Self::Err(value.args.into_iter().next().ok_or(ResponseError::MissingArgs("why"))?),
            "info" => Self::Info(value.args),
            "msg" => {
                let (channel, sender, msg) = (||{
                    let mut args = value.args.into_iter();
                    Some((args.next()?, args.next()?, args.next()?))
                })().ok_or(ResponseError::MissingArgs("channel, sender, msg"))?;
                Self::Msg(channel, sender, msg)
            }
            _ => return Err(ResponseError::UnknownCmd(value.cmd)),
        })
    }
}
