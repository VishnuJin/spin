use anyhow::{bail, Result};
use fermyon_engine::{Config, ExecutionContextBuilder};
use fermyon_http::{HttpEngine, Trigger};
use semver::Version;
use std::{sync::Arc, time::Instant};
use structopt::{clap::AppSettings, StructOpt};
use wact_client::Client;
use wact_core::Entity;

/// Start the Fermyon HTTP runtime.
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Start the default HTTP listener",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]
pub struct Up {
    #[structopt(
        long = "listen",
        default_value = "127.0.0.1:3000",
        help = "IP address and port to listen on"
    )]
    pub address: String,

    /// The target profile to use.
    #[structopt(
        short = "r",
        long = "registry",
        default_value = "http://localhost:8080/v1"
    )]
    pub registry: String,

    /// The registry reference.
    #[structopt(long = "bindle")]
    pub bindle: Option<String>,

    /// The registry reference version.
    #[structopt(long = "bindle-version")]
    pub bindle_version: Option<String>,

    /// The registry reference version.
    #[structopt(long = "local")]
    pub local: Option<String>,

    /// The target profile to use.
    #[structopt(short = "p", long = "profile", default_value = "wasmtime")]
    pub profile: String,
}

impl Up {
    pub async fn run(self) -> Result<()> {
        let start = Instant::now();
        let entrypoint = match self.local {
            Some(e) => e,
            None => {
                let client = Client::new(self.registry).await?;
                let entity = match client
                    .pull(
                        &self.bindle.expect("bindle reference required"),
                        &Version::parse(&self.bindle_version.expect("bindle version required"))?,
                    )
                    .await?
                {
                    Some(e) => e,
                    None => bail!("Cannot pull component from the registry."),
                };

                let component = match entity {
                    Entity::Component(c) => c,
                    Entity::Interface(_) => bail!("Cannot use interface as component."),
                };

                log::info!("Pulled component from the registry: {:?}", component.name());

                component.module().to_str().unwrap().to_string()
            }
        };

        log::info!(
            "Starting the Fermyon HTTP runtime listening on {} using entrypoint {}",
            self.address,
            entrypoint
        );

        let engine =
            ExecutionContextBuilder::build_default(&entrypoint, Config::default()).unwrap();
        let engine = HttpEngine(Arc::new(engine));

        let trigger = Trigger {
            address: self.address,
        };

        log::info!("Total runtime initialization time: {:#?}", start.elapsed());
        trigger.run(engine).await
    }
}