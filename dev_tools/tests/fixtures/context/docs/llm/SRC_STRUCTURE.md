# Fixture source structure

## All functions

| Function | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| src/a.rs::top | src/a.rs::mid, src/b.rs::ghost | comp | func | top-level entry | dev:alpha-overview, llm:alpha-card |
| src/a.rs::mid | src/b.rs::leaf | comp | func | middle helper |  |
| src/b.rs::leaf |  | comp | func | leaf worker | dev:alpha-detail |

## All types

| Type | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| src/a.rs::TopCfg |  | comp | func | config type |  |
