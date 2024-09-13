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
```
<p style="font-size: smaller; color: gray">These are real detections against real cheaters, but the Steam IDs have been substituted for now-deleted bot accounts.</p>

In production, the `-q` flag is used to silence all debug info, leaving only the detection output in stdout. The output is a json array containing serialized Detection objects. The `Detection` struct is used to represent a single detection event. It contains the following fields:

- `tick: u64`: The tick number at which the detection occurred.
- `algorithm: String`: The name of the algorithm which produced the detection.
- `player: u64`: The Steam ID of the player who triggered the detection.
- `data: Value`: A JSON value containing any additional data that the algorithm wishes to store about the detection. This is used to store additional context relevant to the detection.

### Arguments

The program accepts the following arguments:

- `-i <path>`: Specify the path to the demo file to analyze. This argument is required.
- `-q`: Silence all debug info, leaving only the detection output in stdout. Strongly recommended for production use.
- `-c`: Print the number of detections instead of details for every detection. Overridden by `-q`.
- `-a [list of algorithms to run]`: Specify the algorithms to run. If not specified, the default algorithms are run. A list of algorithms can be printed with `-h`.
- `-h`: Print help information and exit.

### Writing your own algorithm

To write your own algorithm, you must implement the `DemoTickEvent` trait. To do this, create a new file in the `src/algorithms/` directory. For example, if you want to write a wallhack detection algorithm, you might create `src/algorithms/wallhack.rs`.

In this file, you must define a struct that implements `DemoTickEvent`. The struct should have a constructor (`new`) and should implement one or more functions from the `DemoTickEvent` trait. These functions return a `Result<Vec<Detection>, Error>`.

You must implement `fn algorithm_name(&self) -> &str` to give your algorithm a name, then use that function to set the algorithm field in every Detection object you return.

To add your algorithm to the project, you must modify `src/main.rs` to include your algorithm in the default list of algorithms, or to include it in the list of available algorithms that can be run with the `-a` flag.

For example, if you wrote a wallhack detection algorithm in `src/algorithms/wallhack.rs`, you would add the following lines to `src/main.rs` to include it in the default list of algorithms:

