use crate::cli::{InjectOptions, Process};

impl Process for InjectOptions {
    async fn process(self) -> Result<u32, String> {
        dbg!("process - inject");
        return Ok(2);
    }
}