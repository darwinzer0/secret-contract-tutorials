# Tutorial: Adding viewing keys to a secret contract

## Introduction

In this tutorial we will demonstrate how to add Viewing Key code to the reminder secret contract that we built in the [Developing your first secret contract](https://learn.figment.io/network-documentation/secret/tutorials/creating-a-secret-contract-from-scratch) tutorial. In that tutorial we implemented code to store and read a private reminder for a user. As implemented, each read of the reminder costs gas, which is not ideal. We will show here how a *viewing key* can be used to implement the same functionality in way that does not require the user to send a gas payment every time they want to read the reminder. 

A viewing key is simply a randomly generated password defined for an address that is stored in the contract. If a query sends a user's address and viewing key together as parameters in the query, then we can use that information to share read-only private data with the user without needed to incur any gas fees.

The viewing key code implemented in this tutorial is based on the implementation used in the SecretSCRT contract: https://github.com/enigmampc/secretSCRT.

## Step 1 - preparing the build environment

To begin you will need to add the following packages to the `Cargo.toml` file:

```toml
secret-toolkit = { git = "https://github.com/enigmampc/secret-toolkit", branch = "debug-print" }
subtle = { version = "2.2.3", default-features = false }
base64 = "0.12.3"
hex = "0.4.2"
sha2 = { version = "0.9.1", default-features = false }
```

## Step 2 - adding viewing key utility

First we will import two source files that define the main ViewingKey struct as well as a couple of utility functions. This code is pulled directly from the [Secret SCRT contract](https://github.com/enigmampc/secretSCRT). Both of these files can be found [here](https://github.com/darwinzer0/secret-contract-tutorials/tree/main/tutorial2). With slight modification you could easily combine these into one file if you wish.

### Add `utils.rs`

```rust
use crate::viewing_key::VIEWING_KEY_SIZE;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use subtle::ConstantTimeEq;

pub fn ct_slice_compare(s1: &[u8], s2: &[u8]) -> bool {
    bool::from(s1.ct_eq(s2))
}

pub fn create_hashed_password(s1: &str) -> [u8; VIEWING_KEY_SIZE] {
    Sha256::digest(s1.as_bytes())
        .as_slice()
        .try_into()
        .expect("Wrong password length")
}
```

This utility crate defines two helper functions: `ct_slice_compare`, which will be used to test if two hashed passwords are the same; and `create_hashed_password`, which creates a hashed password from a random seed using the SHA-256 hash algorithm.

### Add `viewing_key.rs`

```rust
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Env;
use secret_toolkit::crypto::{sha_256, Prng};

use crate::utils::{create_hashed_password, ct_slice_compare};

pub const VIEWING_KEY_SIZE: usize = 32;
pub const VIEWING_KEY_PREFIX: &str = "api_key_";

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ViewingKey(pub String);

impl ViewingKey {
    pub fn check_viewing_key(&self, hashed_pw: &[u8]) -> bool {
        let mine_hashed = create_hashed_password(&self.0);

        ct_slice_compare(&mine_hashed, hashed_pw)
    }

    pub fn new(env: &Env, seed: &[u8], entropy: &[u8]) -> Self {
        // 16 here represents the lengths in bytes of the block height and time.
        let entropy_len = 16 + env.message.sender.len() + entropy.len();
        let mut rng_entropy = Vec::with_capacity(entropy_len);
        rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
        rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
        rng_entropy.extend_from_slice(&env.message.sender.0.as_bytes());
        rng_entropy.extend_from_slice(entropy);

        let mut rng = Prng::new(seed, &rng_entropy);

        let rand_slice = rng.rand_bytes();

        let key = sha_256(&rand_slice);

        Self(VIEWING_KEY_PREFIX.to_string() + &base64::encode(key))
    }

    pub fn to_hashed(&self) -> [u8; VIEWING_KEY_SIZE] {
        create_hashed_password(&self.0)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for ViewingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

This file defines the struct for our viewing key, `ViewingKey`. The `new` method is a constructor that creates a new viewing key given a `seed` and `entropy`. The `seed` is the seed we will use with the pseudorandom number generator, and is a property of the contract that is set when the contract is created. The `entropy` is provided by the user and sent from the client when they want to create a new viewing key. The `entropy` is further extended by adding in the current block height, block time, and the canonical address of the sender. The seed and entropy are used to create an instance of a Prng (a pseudorandom number generator from the secret-toolkit library). We get a slice of random bytes from the random number generator and pass this to the SHA256 algorithm to get our key, which is finally encoded in Base64.

Once a viewing key has been created, the method `check_viewing_key` can be used to test whether a given hashed password matches the viewing key.

### Integrating into your contract

After you have added `viewing_key.rs` and `utils.rs` to the `src/` directory, in `src/lib.rs` you will need to add them as modules in the project. To do that add these lines near the top:

```rust
mod viewing_key;
mod u256_math;
```

In the contract, we also need to add a pseudorandom number generator seed in our config. In `state.rs` add this to our `State` struct: 

```rust
    pub prng_seed: Vec<u8>,
```

In `msg.rs` update the `InitMsg` struct by adding:

```rust
    pub prng_seed: Binary,
```

Finally, in the `init` function in `contract.rs` update the initialization of the config to the following:

```rust
    let config = State {
        max_size,
        reminder_count: 0_u64,
        prng_seed: sha_256(&msg.prng_seed.0).to_vec(),
    };
```

## Step 3 - generating a viewing key

Next we need to add a Handle function to generate a viewing key for a user. In `msg.rs` we add the following to HandleMsg:

```rust
    GenerateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
```

When we create a new key the client sends in some entropy to contributes to the randomness of the viewing key. The client should create a random string and pass it in with this parameter. Padding is simply an optional parameter that can be used to obfuscate the length of the entropy string.

Then, we add a new function in `contract.rs` to generate the key. In `handle` add:

```rust
    HandleMsg::GenerateViewingKey { entropy, .. } => try_generate_viewing_key(deps, env, entropy),
```

And add `try_generate_viewing_key`:

```rust
pub fn try_generate_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: String,
) -> StdResult<HandleResponse> {
    let config: State = load(&mut deps.storage, CONFIG_KEY)?;
    let prng_seed = config.prng_seed;

    let key = ViewingKey::new(&env, &prng_seed, (&entropy).as_ref());

    let message_sender = deps.api.canonical_address(&env.message.sender)?;

    write_viewing_key(&mut deps.storage, &message_sender, &key);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::GenerateViewingKey { 
            key,
        })?),
    })
}
```

## Step 4 - creating authenticated queries

### Updating msg.rs

Now we can update our Query messages to include authenticated queries. For example, let's say we want to implement Read as a query instead of a execute function, so we don't need to pay gas fees with every read. In `QueryMsg` in `msg.rs` add:

```rust
    Read {
        address: HumanAddr,
        key: String,
    }
```

When we make a Read query we pass in the address of the querier using their human-friendly address (i.e., `secret...`) and the viewing key string.

To easily access validation parameters for authenticated queries we can add an implementation block to our `QueryMsg` struct by adding the following below its declaration:

```rust
impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Read { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}
```

Then we define the `Read` response in the `QueryAnswer` enum. We also create a `ViewingKeyError` response to be sent whenever an authenticated query sends the wrong viewing key:

```rust
    Read {
        status: String,
        reminder: Option<String>,
        timestamp: Option<u64>,
    },
    ViewingKeyError {
        msg: String,
    },
```

### Updating contract.rs

Now we turn to `contract.rs` to update our `query` function. It is easiest to use a helper function to deal with all of the authenticated queries. Add the following line at the bottom of the `match msg` block in `query`:

```rust
    _ => authenticated_queries(deps, msg),
```

This means that any queries messages not already handled will be passed to a function called `authenticated_queries`. We define that as follows:

```rust
fn authenticated_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    let (addresses, key) = msg.get_validation_params();

    for address in addresses {
        let canonical_addr = deps.api.canonical_address(address)?;

        let expected_key = read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
        } else if key.check_viewing_key(expected_key.unwrap().as_slice()) {

            return match msg {
                QueryMsg::Read { address, .. } =>
                    query_read(&deps, &address),
                _ => panic!("This query type does not require authentication"),
            };
        }
    }

    Ok(to_binary(&QueryAnswer::ViewingKeyError {
        msg: "Wrong viewing key for this address or viewing key not set".to_string(),
    })?)
}
```

This code checks that the correct viewing key has been sent for the given address(es). If no viewing key has been set, we don't want that information to leak based on the time of execution, so we essentially run a noop to cycle through the same time that it would take to check the key if it did exist. If the key matches, then we can handle the specific type of query that was sent (in our case Read). If the viewing key doesn't not match or was not set, then we return a `ViewingKeyError` response message. 

Now we can implement the `query_read` function. It is very similar to our `try_read` handle function from before, but instead of getting the sender address from `deps.api` we use the address that was sent as a query parameter:

```rust
fn query_read<S: Storage, A: Api, Q: Querier>(
	deps: &Extern<S, A, Q>,
	address: &HumanAddr,
) -> StdResult<Binary> {
	let status: String;
    let mut reminder: Option<String> = None;
    let mut timestamp: Option<u64> = None;

    let sender_address = deps.api.canonical_address(&address)?;

    // read the reminder from storage
    let result: Option<Reminder> = may_load(&mut deps.storage, &sender_address.as_slice().to_vec()).ok().unwrap();
    match result {
        // set all response field values
        Some(stored_reminder) => {
            status = String::from("Reminder found.");
            reminder = String::from_utf8(stored_reminder.content).ok();
            timestamp = Some(stored_reminder.timestamp);
        }
        // unless there's an error
        None => { status = String::from("Reminder not found."); }
    };

    to_binary(&QueryAnswer::Read{ status, reminder, timestamp })
}
```

Now you can read the reminder as many times as you want without paying any scrt!

## About the author

This tutorial was written by Ben Adams, a senior lecturer in computer science and software engineering at the University of Canterbury in New Zealand.

<div class="cc">
<a rel="license" href="http://creativecommons.org/licenses/by/4.0/"><img alt="Creative Commons License" style="border-width:0" src="https://i.creativecommons.org/l/by/4.0/88x31.png" /></a><br />This work is licensed under a <a rel="license" href="http://creativecommons.org/licenses/by/4.0/">Creative Commons Attribution 4.0 International License</a>.
</div>