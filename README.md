# torpor

> Torpor is a state of decreased (physiological) activity

Our `torpor` inflicts precisely that on every visitor's browser. An anti-abuse **proof-of-work**, built on a [Rivest–Shamir–Wagner *time-lock puzzle*](https://people.csail.mit.edu/rivest/pubs/RSW96.pdf), that forces a fixed, unavoidable stretch of slow work before the request is let through.

Unlike common hashcash PoWs, the work is inherently **sequential**: a single challenge cannot be sped up by more cores, a GPU, or SIMD, so an attacker gets the smallest possible edge over an ordinary browser. The server issues and verifies each challenge for next to nothing; the client does the slow part in [WASM](https://webassembly.org/).

## How it works

The server picks a random base `x` and difficulty `t`. Using the secret factorization of its modulus `n = p * q` it precomputes the answer `y = x^(2^t) mod n` in `O(log t)`.
The client has to compute the same `y` the slow way. `t` sequential modular squarings.
The server verifies with a single equality check.

## Server

```rust
use std::{net::IpAddr, time::Duration};
use torpor::Difficulty;
use torpor::timelock::server::{ChallengeStore, Trapdoor, Verify};

// Generate the 2048-bit trapdoor once. p, q should never leave the server
let mut store: ChallengeStore<IpAddr> =
    ChallengeStore::new(Trapdoor::generate(2048), Duration::from_secs(60));

// Keyed per client. A re-issue replaces that client's old challenge
let challenge = store.issue(client_ip, Difficulty(300_000)); // Adjust the difficulty to one of your choosing
// send c.modulus.to_hex(), c.base.to_hex(), c.difficulty.0 to the browser

// Verify the hex answer it returns.
let answer = torpor::Wide::from_hex(&answer_hex)?;
match store.verify(&client_ip, &answer) {
    Verify::Accepted => { /* let the request through */ }
    Verify::Wrong | Verify::UnknownOrExpired => { /* reject */ }
}
```

## Client

Zero build for consumers. Load it from [jsDelivr](https://www.jsdelivr.com/) and solve.
The work runs in one Web Worker, so the page stays responsive:

```html
<script type="module">
  import { Torpor } from 'https://cdn.jsdelivr.net/gh/41Baloo/torpor@v0.1.0/web/torpor.js';

  const pow = new Torpor({ modulus, base, difficulty });    // hex, hex, number from the server
  pow.onProgress = ({ progress, rate, etaMs }) => { /* drive a bar / ETA */ };
  const { answer } = await pow.solve();                     // Return answer to the server
  // pow.cancel() stops immediately
</script>
```

To self-host, serve `web/torpor.js` + `web/pkg/` from your own origin instead

## Build

```bash
wasm-pack build --release --target web --out-dir web/pkg --out-name torpor && rm -f web/pkg/.gitignore
```
