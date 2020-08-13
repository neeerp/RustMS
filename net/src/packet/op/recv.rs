#[derive(FromPrimitive)]
pub enum RecvOpcode {
    LoginCredentials = 0x01,
    LoginStarted = 0x23,
}
