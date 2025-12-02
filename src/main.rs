use tscrapingbot_rs::{BoxError, run};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), BoxError> {
    run().await
}
