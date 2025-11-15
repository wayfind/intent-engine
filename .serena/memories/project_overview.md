# Intent-Engine Project Overview

## Project Purpose
Intent-Engine is an **external long-term memory system for AI assistants** and a **strategic task management system**.

Key features:
- **Strategic Intent Layer**: Focus on What (what to do) and Why (why do it), not How
- **Cross-session Memory**: Persisted to SQLite, can restore complete context anytime
- **Hierarchical Task Tree**: Supports unlimited parent-child task levels, natural problem decomposition
- **Decision History**: Every key decision recorded as event stream, traceable and reviewable
- **AI Native**: CLI + JSON + MCP protocol, deeply optimized for AI toolchains

## Tech Stack

- **Language**: Rust 2021 Edition
- **CLI Framework**: clap 4.5 (declarative command-line)
- **Database**: SQLite + sqlx 0.8 (async queries)
- **Full-text Search**: SQLite FTS5 (millisecond-level search)
- **Async Runtime**: tokio 1.35
- **Serialization**: serde 1.0 + serde_json 1.0
- **Error Handling**: thiserror 2.0 + anyhow 1.0
- **Date/Time**: chrono 0.4

## Binary

- Main binary: `ie` (intent-engine CLI)
- Entry point: `src/main.rs`

## Version
Current version: 0.3.4 (as per Cargo.toml)
Interface spec version: 0.3
