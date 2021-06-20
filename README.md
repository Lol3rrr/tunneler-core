# Tunneler-Core
[![Docs](https://docs.rs/tunneler-core/badge.svg)](https://docs.rs/tunneler-core/)
[![Crates.io](https://img.shields.io/crates/v/tunneler-core.svg)](https://crates.io/crates/tunneler-core)
The Core functionality of the Tunneler-Software

## Description
This package provides the Core functionality regarding the Client and Server
side of the Tunneler-Software as well as allowing you to integrate this Software/Protocol
into your own Projects where needed.

## Features
Name | Default | Description
--- | --- | ---
client | enabled | All the Client related code
server | enabled | All the Server related code
logging | enabled | Enables all the log related parts using the `log` crate
trace | enabled | Enables all the tracing-related parts using the `tracing` and `tracing-futures` crates
