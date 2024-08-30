# Demo Analysis Template (Rust) (VERY WIP FOR NOW)

This repo contains a Rust template for TF2 demo analysis, as well as some very basic examples.

## Structure

As much of the existing work for demo analysis already exists in Rust, we will continue using Rust for our demo analysis for the time being.

main.rs -> ticker.rs -> algorithms in /src/algorithms

The final product of this repo is an executable that accepts a demo file as input, and returns a json string containing metadata related to the demo. All algorithms for MegaAntiCheat will be implemented in a private fork; this repo merely acts as a public-facing template.

## How to write a cheat detection algorithm

The `DemoTickEvent` trait in `main.rs` is used as the base for any cheat detection algorithm. 



## Output


