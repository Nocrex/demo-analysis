# Demo Analysis Template (Rust) (VERY WIP FOR NOW)

This repo contains a Rust template for TF2 demo analysis, as well as some very basic examples.

## Structure

As much of the existing work for demo analysis already exists in Rust, we will continue using Rust for our demo analysis for the time being.

Rust template -> Rust events -> Rust cheat detection algorithms

The final product of any cheat detection algorithm that uses this template is an executable that accepts a demo file as input, and returns a json string containing metadata related to the demo.

## Events

When a demo is analysed, several events are fired. Cheat detection algorithms will listen for these events and do their analysis. Since cheat detection is done on a per-player basis, all functions fire once for each player.

WIP: The params specified here are just loose guidelines for what data is expected. Exact types will be specified later as things are implemented.

### `DemoStartEvent`

Fired once prior to all other analysis. Intended for initialisation.

### `DemoTickEvent`

Params: playerData{}

Fired on every tick for each player for which data on that player exists (PVS will exclude many ticks on POV demos).

### `DemoWeaponFireEvent`

Params: playerData{}

Fired on every tick in which a player fires their weapon.

### `DemoDamageEvent`

Params: attackerData{}, victimData{}, damageEventData{}

Fired on every tick that has a damage event. Must include the data of both the attacker and the victim.

### `DemoKillEvent`

Params: attackerData{}, victimData{}, damageEventData{}, killEventData{}

Fired for every tick that has a kill event. Must include the data of both the attacker and the victim.

### `DemoEndEvent`

Fired once after all other analysis has concluded. Intended to be used when analysing data of interest accumulated across multiple other events.

For example, imagine a DemoKillEvent implementation that when fired increments two counters: one counter increments on every kill, but the other only increments if the kill was via a critical hit.

A DemoEnd event can then be implemented to compare the final values of those counters and output a final analysis on what players had unusually high percentages of critical kills.

## Helper functions

### `GetSurroundingTicks(u32 tickNum, u32 forward, u32 backward)`

Gets data for the ticks surrounding the tick that's being analysed. Cheat detection algorithms need multiple ticks either side of the actual cheating to detect suspicious behaviour, and requiring the implementation to request those extra ticks when needed prevents unnecessary iteration. 

### `CalculateFOV`

Accepts the positions and viewangles of two players, an attacker and a victim, and returns the minimum size of FOV circle required to align the crosshair with a hitbox (in degrees).



## Output


