use crate::cli::{Process, RunuvOptions};

impl Process for RunuvOptions {
    async fn process(self) -> Result<u32, String> {
        dbg!("process - runuv");
        return Ok(2);
    }
}