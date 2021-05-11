/*
 * Copyright 2018 Intel Corporation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * ------------------------------------------------------------------------------
 */

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate log4rs;
extern crate rand;
extern crate sawtooth_sdk;

pub mod config;
pub mod engine;
pub mod timing;
pub mod storage;
pub mod state;
pub mod node;

use std::process;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

use engine::PolyShardEngine;
use sawtooth_sdk::consensus::zmq_driver::ZmqDriver;

fn main() {
    let matches = clap_app!(("devmode-engine-rust") =>
        (version: crate_version!())
        (about: "PolyShard Consensus Engine (Rust)")
        (@arg connect: -C --connect +takes_value
         "connection endpoint for validator")
        (@arg verbose: -v --verbose +multiple
         "increase output verbosity"))
    .get_matches();

    let endpoint = matches
        .value_of("connect")
        .unwrap_or("tcp://localhost:5050");

    let console_log_level;
    match matches.occurrences_of("verbose") {
        0 => console_log_level = LogLevelFilter::Warn,
        1 => console_log_level = LogLevelFilter::Info,
        2 => console_log_level = LogLevelFilter::Debug,
        _ => console_log_level = LogLevelFilter::Trace,
    }

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{h({l:5.5})} | {({M}:{L}):20.20} | {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(console_log_level))
        .unwrap_or_else(|err| {
            error!("{}", err);
            process::exit(1);
        });

    log4rs::init_config(config).unwrap_or_else(|err| {
        error!("{}", err);
        process::exit(1);
    });

    info!("Sawtooth PolyShard Engine ({})", env!("CARGO_PKG_VERSION"));

    let mut polyshard_config = config::PolyShardConfig::default();

    let polyshard_engine = PolyShardEngine::new(polyshard_config);


    let (driver, _stop) = ZmqDriver::new();
    driver
        .start(endpoint, polyshard_engine)
        .unwrap_or_else(|err| {
            error!("{}", err);
            process::exit(1);
        });
}
