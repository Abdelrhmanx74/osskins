use std::sync::Mutex;

pub struct ConfigLock(pub Mutex<()>);

impl ConfigLock {
  pub fn new() -> Self {
    Self(Mutex::new(()))
  }
}
