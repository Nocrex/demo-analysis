# Demo Analysis Template (Rust)

This repo contains a Rust template for TF2 demo analysis, as well as some very basic implementations.

This repo produces an executable that accepts a demo file as input, and returns a json string containing metadata related to the demo. All algorithms for MegaAntiCheat will be implemented in a private fork; this repo merely acts as a public-facing template.

## How to use this template.

### Getting started

After setting up your Rust developer environment, check the program executes correctly with the command `cargo run --release -- -i "path/to/demo.dem"`. The --release flag dramatically speeds up the program (~2x), so its use is recommended even during development.

Example output: 
```Map: pl_borneo
Duration: 00:11:28.038 (45892 ticks)
User: megascatterbomb (part 2 soon)
Server: 117.120.11.35:6996
Starting analysis...
Processing tick 3238 (42654 remaining, 3237 tps)
Processing tick 5838 (40054 remaining, 2918 tps)
Processing tick 8750 (37142 remaining, 2915 tps)
Processing tick 12013 (33879 remaining, 3002 tps)
Processing tick 14829 (31063 remaining, 2964 tps)
Processing tick 16994 (28898 remaining, 2831 tps)
Processing tick 19277 (26615 remaining, 2752 tps)
Processing tick 21628 (24264 remaining, 2702 tps)
Processing tick 24571 (21321 remaining, 2729 tps)
Processing tick 27254 (18638 remaining, 2724 tps)
Processing tick 29753 (16139 remaining, 2703 tps)
Processing tick 33029 (12863 remaining, 2751 tps)
Processing tick 36511 (9381 remaining, 2807 tps)
Processing tick 39226 (6666 remaining, 2800 tps)
Processing tick 43498 (2394 remaining, 2898 tps)
Map: pl_borneo
Duration: 00:11:28.038 (45892 ticks)
User: megascatterbomb (part 2 soon)
Server: 117.120.11.35:6996
Done! (Processed 45892 ticks in 15.64 seconds averaging 2934.88 tps)
[]
```

The empty array at the bottom means there were no detections. If there were detections, they would look something like this:

```
[
  {
    "tick": 8466,
    "algorithm": "viewangles_180degrees",
    "player": 76561199776113179,
    "data": {
      "pa_delta": -180.0,
      "va_delta": -58.768341064453125
    }
  },
  {
    "tick": 8466,
    "algorithm": "viewangles_180degrees",
    "player": 76561199775364340,
    "data": {
      "pa_delta": -180.0,
      "va_delta": 33.07917404174805
    }
  },
  {
    "tick": 8469,
    "algorithm": "viewangles_180degrees",
    "player": 76561199774314308,
    "data": {
      "pa_delta": -180.0,
      "va_delta": 33.079193115234375
    }
  },
  {
    "tick": 8469,
    "algorithm": "viewangles_180degrees",
    "player": 76561199776113179,
    "data": {
      "pa_delta": 180.0,
      "va_delta": -134.0762424468994
    }
  },
  {
    "tick": 8469,
    "algorithm": "viewangles_180degrees",
    "player": 76561199775364340,
    "data": {
      "pa_delta": 180.0,
      "va_delta": 33.079185485839844
    }
  },
  {
    "tick": 8474,
    "algorithm": "viewangles_180degrees",
    "player": 76561199774314308,
    "data": {
      "pa_delta": 180.0,
      "va_delta": 33.079193115234375
    }
  },
  {
    "tick": 8474,
    "algorithm": "viewangles_180degrees",
    "player": 76561199776113179,
    "data": {
      "pa_delta": -180.0,
      "va_delta": 20.762466430664062
    }
  },
  {
    "tick": 8474,
    "algorithm": "viewangles_180degrees",
    "player": 76561199775364340,
    "data": {
      "pa_delta": -180.0,
      "va_delta": 49.61875915527344
    }
  }
]
```
These are real detections against real cheaters, but the Steam IDs have been substituted for now-deleted bot accounts.

In production, the `-q` flag is used to silence all debug info, leaving only the detection output in stdout. 

### Output

The output is a json array containing serialized Detection objects. The `Detection` struct is used to represent a single detection event. It contains the following fields:

- `tick: u64`: The tick number at which the detection occurred.
- `algorithm: String`: The name of the algorithm which produced the detection.
- `player: u64`: The Steam ID of the player who triggered the detection.
- `data: Value`: A JSON value containing any additional data that the algorithm wishes to store about the detection. This is used to store additional context relevant to the detection.

### Arguments

The program accepts the following arguments:

- `-i <path>`: Specify the path to the demo file to analyze. This argument is required.
- `-q`: Silence all debug info, leaving only the detection output in stdout. Required for production use.
- `-p`: Same as `-q`, but prettifies the output. Convenient for manual inspection of the output.
- `-c`: Print the number of detections instead of details for every detection. Overridden by `-q`.
- `-a <algorithm> [-a <algorithm>]...`: Specify the algorithms to run. If not specified, the default algorithms are run.
- `-h`: Print help information and exit.

### Writing your own algorithm

This section describes the structure of a cheat detection algorithm. You can also view a complete algorithm with supporting comments at `src/algorithms/viewangles_180degrees.rs`.

To write your own algorithm, you must implement the `CheatAlgorithm` trait. To do this, create a new file in the `src/algorithms/` directory. For example, if you want to detect 180 degree snaps, you might create `src/algorithms/viewangles_180degrees.rs`. In this file, you can define whatever structs, types etc you need to create your algorithm. At minimum, you need to implement some of the functions in `CheatAlgorithm`:

- `default(&self) -> bool` (REQUIRED): Should this algorithm run by default if -a isn't specified?
- `algorithm_name(&self) -> &str` (REQUIRED): Return your algorithm's name here. Best practice is to match the filename.
- `init(&mut self) -> Result<Vec<Detection>, Error>`: Called before any other events. Use this instead of your object's constructor when performing any non-ephemeral actions e.g. modifying files.
- `on_tick(&mut self, tick: Value) -> Result<Vec<Detection>, Error>`: Called for each tick. The json state for the tick is passed in as a json Value.
- `finish(&mut self) -> Result<Vec<Detection>, Error>`: Called after all other events. Use for cleaning up or for aggregate analysis.

The functions that return `Result<Vec<Detection>, Error>` are the entry points for your actual algorithm. Your task is to process the incoming data and produce Detection objects for each event where cheating is suspected.

The incoming data is provided as a json value via `CheatAlgorithm::on_tick`. To understand the structure of this object, try `cargo run --release -i "path/to/demo.dem" -a write_to_file` to write all the json states to one large file. Each tick is written to a new line.

To register a detection, include it in the vector that's returned at the end of any detection function. Detections don't have to be returned in the same function call that the relevant data is introduced; you can store Detections elsewhere and return them all in CheatAlgorithm::finish() if you want, but make sure all the Detection objects you want to return are returned before the program terminates. This is a good pattern for aggregate detection methods e.g. crit hack detection.

If you don't have any detections to return, just return the empty vector.