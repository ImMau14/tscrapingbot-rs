use tscrapingbot_rs::BoxError;
use tscrapingbot_rs::run;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), BoxError> {
    run().await
}
