//! `ratiomaster-core` is the core library for RatioMaster, a BitTorrent tracker
//! communication tool.
//!
//! This crate provides:
//! - [`bencode`] - BEncode codec (encoder/decoder) for the BitTorrent serialization format
//! - [`torrent`] - .torrent file parser and metadata types
//! - [`encoding`] - URL encoding utilities for tracker communication
//! - [`network`] - Async TCP, raw HTTP client, and local IP detection
//! - [`proxy`] - SOCKS4/4a/5 and HTTP CONNECT proxy support
//! - [`tracker`] - Tracker announce, response parsing, and scrape
//! - [`client`] - Client emulation profiles, peer ID and key generation
//! - [`engine`] - Core announce engine, batch operations, and speed simulation
//! - [`config`] - Application configuration, sessions, custom profiles, and version checking

pub mod bencode;
pub mod client;
pub mod config;
pub mod encoding;
pub mod engine;
pub mod error;
pub mod network;
pub mod proxy;
pub mod torrent;
pub mod tracker;
