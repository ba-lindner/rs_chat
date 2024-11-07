use crate::package_enum;

package_enum! {
    pub enum Response {
        Ack("ack"),
        Err("err" => why),
        Info("info" => [data]),
        Msg("msg" => channel, name, msg),
    }
}

impl Response {
    pub fn is_bad(&self) -> bool {
        matches!(self, Self::Err { .. })
    }
}
