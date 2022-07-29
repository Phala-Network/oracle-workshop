# Fat Contract Oracle Workshop

_First created for Polkadot Decoded 2022 with the subtitle "The Web3 infrastructure beyond smart contracts: Build an oracle in 15 mins with ink!"_ ([Slides](https://docs.google.com/presentation/d/1HjmQCSvwpc7gwaCU2W_5yHA3gTIrg49SpSUZ4gLZwD8/edit?usp=sharing))

## What you can learn

There are the beginner challenge and the advanced challenge in the workshop. In the beginner challenge, you are going to play with the oracle built on Phala Fat Contract. In the advanced challenge, you are going to learn how to build an oracle that:

1. links off-chain identity to blockchain
2. sends HTTP requests to verify off-chain data
3. gives out [POAP NFT](https://poap.xyz/) rewards
4. is written in [ink!](https://ink.substrate.io/)
5. (and can be built in 15 mins)

## Bonus

When you have solved a challenge successfully, you can earn a beautiful POAP.

| Beginner POAP | Advanced PAOP |
| -------- | -------- |
| ![](https://i.imgur.com/mVNh6Nh.png)     | ![](https://i.imgur.com/kZEquyA.png)     |

---

# Beginner challenge

Before a deep dive into how an oracle is made, let's try out a very simple oracle demo first. In this demo, you will be asked to post a message to Github to prove your ownership of the account. The Fat Contract will verify your post from Github. Once the verification is passed, you will win a nice POAP as a bonus!

<img src="https://i.imgur.com/mVNh6Nh.png" width="100"/>

## Step-by-step

In this challenge, you will interact with the workshop DApp in your browser. Before starting, please make sure you have:

1. [Polkadot.js Browser Extension](https://polkadot.js.org/extension/)
3. [Github](https://github.com/) account

> Some Polkadot.js Extension compatible extensions may also work, but this tutorial is only tested with the official extension.

If you haven't done it yet, please [generate an account](https://wiki.polkadot.network/docs/learn-account-generation#polkadotjs-browser-extension) in your Polkadot.js Extension. Otherwise, we are ready to go!

Open the [Workshop DApp](https://phala-decoded-2022.netlify.app), and enter the _Easy Challenge_ page.

![](https://i.imgur.com/ytxwnnJ.png)

On the _Easy Challenge_ page, the browser will immediately pop up an _Authorize_ window. Click _Yes_ to allow the authorization. Then you can click the _Select Account_ drop-down to connect to an account. 

On this page, you can request the faucet to get some test tokens by _Get 10 PHA_ button (under the drop-down). Please do if you haven't done it yet. The operations below require tokens as the transaction fee.

![](https://i.imgur.com/1vMK0Lz.png)

Now, let's click _Sign Certificate_. It will ask you to sign a _certificate_ that is used to interact with the contracts. Once it's finished, it will show you the DApp UI like below.

![](https://i.imgur.com/Ak6kquP.png)

The DApp asks you to create a Github Gist with the given text. You can follow the Github link on the page to create a gist. You should paste the text it gives you as the content of the gist, and submit it. The filename and the title don't matter. Both public and private gist work.

![](https://i.imgur.com/sFuPV2U.png)

Once the gist is created, open it in the _raw format_ (as highlighted in the screenshot below). Then copy the URL of the raw gist, and paste it to section 2 in the DApp. Please note that the raw gist URL should match the following pattern:

```
https://gist.githubusercontent.com/<username>/.../raw/...
```

Then, click _Verify_ to submit the URL as proof. If the verification is passed, you will be asked to sign a transaction to redeem the PAOP code. The transaction may take up to half a minute to complete. When you get the prompt saying the transaction is finalized, you can follow the _FatBadges_ link in section 3 to check your redeem code.

On this page, you will need to sign the certificate again. Then click _Load_ button, and it will show you your PAOP redeem code as well as the basic stats of the challenges.

![](https://i.imgur.com/mFG9WpX.png)

Congratulations! Now you should be able to use the redeem code to get your shining NFT!

Want to know how it works? We will cover this in the next section.

## Build an oracle in Fat Contract

### Prerequests

To read this section, it's suggested to have a basic understanding of the following concepts:

1. Smart contracts
2. Oracles
3. Blockchain Consensus

### The way to scale oracles

Existing oracles don't scale. For instance, ChainLink is the most commonly used oracle. It supports only 12 EVM blockchains, and they struggle to add long-tail support. On the other hand, existing oracles often serve very limited data sources and functions (price feed and VRF). Though there are rich data available on the internet, none of the oracle is flexible enough to process this data and make it useful to blockchain apps.

The key to scale oracle is the ability to make its logic programmable. Thinking about building a customized oracle as easy as writing smart contracts, it would be easy to support all the long-tail blockchain and use cases.

Unfortunately, traditional oracles are not good at this because of their technical limitation. Oracle brings outside information to a blockchain. It must run off-chain because a blockchain can never access the data beyond its consensus system by itself. Without the consensus systems, the security guarantee disappears of a sudden. As a result, a decentralized off-chain project needs to take care to design the mechanism to ensure the correctness and the liveness of the network. Often, we cannot always find the mechanism that applies to all kinds of logic. Not to mention we may need to spend from months to years to implement it.

Is it possible to build a scalable oracle efficiently at all? It turns out possible, but we need an off-chain execution environment with:

1. Internet access: it enables access to all the data available around the world
2. Easy integration: can easily expand to long-tail blockchains in hours
3. Off-chain security: running off-chain, but still as secure as on-chain apps

Fat Contract is designed to meet all these requirements! As the decentralized cloud for Web3, Phala allows you to deploy the customized program to access the internet and report data to any blockchain.

## Fat Contract introduction

Fat Contract is the programming model designed for Phala cloud computation. It has some similarities to smart contracts but is fundamentally different from smart contracts.

To help understand the programming model, let's first learn how Phala works. Phala Network is a network with thousands of secure off-chain workers. The workers are available to deploy apps. Unlike the fully redundant nodes in blockchain, Phala workers run their app in parallel. The developer can pay one or more workers to deploy their app, just like how they deploy apps on the traditional serverless cloud.

This is possible because the workers are secure enough to run apps separately without involving blockchain consensus. In other words, Fat Contract is fully off-chain computation. This gives us the following advantages:

1. Support internet access: Fat Contract provides API to send HTTP requests.
2. Low latency and CPU-intensive computation
3. Keep secrets: states and communications are protected by default

> Wanna know why Phala Network's workers are secure and confidentiality preserving? Please check out [wiki](https://wiki.phala.network/en-us/learn/phala-blockchain/blockchain-detail/).

With the above features, we can build decentralized oracles as easily as writing a backend app. In fact, in the advanced challenge, we are going to show you how to build and deploy a customized oracle in 15 mins.

### Basics

Fat Contract is based on [Parity ink!](https://ink.substrate.io/) and fully compatible with ink!. It has some special extensions and differences in usage to better support the unique features. Most of the time, developing Fat Contract is the same as writing ink! smart contract. So we strongly suggest learning ink! with [the official documentation](https://ink.substrate.io/) first. In this section, we will only cover the basic structure of a contract.

Let's look into the similar part first. In a typical ink! contract, you are going to define the storage and the methods of a smart contract:

```rust
#[ink(storage)]
pub struct EasyOracle {
    admin: AccountId,
    linked_users: Mapping<String, ()>,
}


impl EasyOracle {
    #[ink(constructor)]
    pub fn new() -> Self {
        let admin = Self::env().caller();
        ink_lang::utils::initialize_contract(|this: &mut Self| {
            this.admin = admin;
        })
    }

    #[ink(message)]
    fn admin(&self) -> AccountId {
        self.admin.clone()
    }
    
    #[ink(message)]
    pub fn redeem(&mut self, attestation: Attestation) -> Result<()> {
        // ...
    }
}
```

This example is taken from the [`EasyOracle`](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/easy_oracle/lib.rs) contract. In the code above, we have defined a contract with two storage items accessible in the contract methods. It has three methods. The first one, `new()` is a **constructor** to instantiate a contract. In the constructor, we save the caller as the admin of the contract in the storage. The second method `admin()` is a **query** to return the admin account. Queries can read the storage, but cannot write to the storage (notice the immutable `&self` reference). The third method `redeem()` is a **transaction** method (or called "command" in Fat Contract). Transaction methods can read and write the storage.

It's important to understand the types of methods. **Constructors** and **transaction** methods are only triggered by on-chain transactions. Although the benefit is you can write to the storage, they are slow and expensive, because you always need to send a transaction. Additionally, advanced features like HTTP requests are not available in these methods.

The most interesting part of Fat Contract is the **queries**. Despite the limitation that you can only read the storage, queries give you a lot of benefits:

1. Access to HTTP requests
2. Low latency: the queries are sent to the worker directly and you can get the response immediately, without waiting for blocks
3. Free to call: queries doesn't charge any gas fee

We are going to combine queries and transactions to build the oracle.

### The primitives to build an oracle

The [`EasyOracle`](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/easy_oracle/lib.rs) Fat Contract asks you to post a special statement on Github to verify your ownership of the Github account. By sending a statement like the below, we can ensure the Github account is controlled by yourself:

```
This gist is owned by address: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
```

#### HTTP request

To verify the ownership, the Fat Contract needs to send an HTTP request to the Github Gist server, and check if the content of the gist matches the caller. This can be done in a query like this:

```rust
// Verify the URL
let gist_url = parse_gist_url(&url)?;
// Fetch the gist content
let response = http_get!(url);
if response.status_code != 200 {
    return Err(Error::RequestFailed);
}
let body = resposne.body;
```

The `http_get!` macro is provided by the Fat Contract API [`pink_extension::http_get`](https://docs.rs/pink-extension/0.1.9/pink_extension/macro.http_get.html). It allows the developer to send a `GET` or `POST` request in queries. If it's not in a query, the execution will fail because it violates the determinism rule.

#### Attestation

We can send the HTTP request and verify the response in a query. However, it's not allowed to mutate the contract storage. How can we bring the verification result back to the blockchain to trigger the next step logic?

The answer is [**off-chain attestation**](https://ethereum.org/en/decentralized-identity/#off-chain-attestations). This is a useful pattern that allows users to submit data from queries to the blockchain (to a transaction method, or even an external independent blockchain).

Fat Contract provides the `attestation` utils to easily generate and verify the attestation.

```rust
// Generate a key pair
let (generator, verifier) = attestation::create(b"salt");

// In a query
let payload = SomeData { ... };
let cert = generator.sign(payload)?;

// In an on-chain transaction
if let Some(payload) = verifier.verify_as(cert)? {
    // Verification passed
}
```

Under the hood, an attestation is just a payload signed with a private key. The private key is usually generated by the contract constructor. As only the Fat Contract holds the private key, the signature proves that the data in the payload is created by the contract, and the integrity is guaranteed. When we want to verify the attestation, we simply verify the signature.


#### Access control

Access control is a special feature of Fat Contract. In Fat Contract, the states and the communication are confidentiality-preserving by default. Users can only read encrypted data from the blockchain but cannot guess the plain text. The only way to reveal data to the user is by queries.

In Fat Contract, queries are signed by the user's wallet. This makes it possible to check the role of the user before responding to the query. We can write an easy access control logic like this:

```rust
if self.env().caller() != self.admin {
    return Err(Error::BadOrigin);
}
return self.actual_data;
```

We are going to leverage this feature to store some POAP redeem codes on the blockchain, and distribute the code to the challenge winners only.

### Put everything together

With the HTTP request, off-chain attestation, and access control, we can finally build a full oracle that can check your ownership of a Github account, and produce proof to redeem a POAP code on the blockchain.

To learn more about the implementation, we suggest reading the following Fat Contracts:

1. [`EasyOracle`](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/easy_oracle/lib.rs): The oracle to attest your Github ownership
2. [`FatBadges`](https://github.com/Phala-Network/oracle-workshop/blob/master/fat_badges/lib.rs): The contract to distribute POAP NFT "badges" to challenge winners.

To get started, please check the tutorial for the Advanced Challenge.

## Resources

- Decoded Workshop DApp: <https://phala-decoded-2022.netlify.app>
- Github Repo: <https://github.com/Phala-Network/oracle-workshop>
- Understand the certificate: _TODO_
- Support
    - Discord (dev group): <https://discord.gg/phala>
    - Polkadot StackExchange (tag with `phala` and `fat-contract`): <https://substrate.stackexchange.com/>

---

# Advanced Challenge

At this point, you should be already familiar with the basics of Fat Contract. If not, please go back to the Beginner Challenge section.

In the advanced challenge, you are going to learn:

- How to build your oracle in Fat Contract
- Deploy the oracle to the Phala Testnet
- Use Fat Contract UI to play with your contracts

And finally, once your on-chain submission is successful, you are going to earn a nice Advanced Challenge Winner PAOP!

<img src="https://i.imgur.com/kZEquyA.png" width="100"/>

## Step-by-step

### Prerequets

You need the Polkadot.js browser extension as required in the Beginner Challenge. Additionally, this challenge requires you to install the development environment.

An operating system of macOS or Linux systems like Ubuntu 20.04/22.04 is recommended for the workshop.

- For macOS users, we recommend using the *Homebrew* package manager to install the dependencies
- For other Linux distribution users, use the package manager with the system like Apt/Yum

The following toolchains are needed:

- Rust toolchain
    - Install rustup, rustup is the "package manager" of different versions of Rust compilers: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - This will install `rustup` and `cargo`
- Ink! Contract toolchain
    - Install [binaryen](https://github.com/WebAssembly/binaryen) with
        - Homebrew for macOS: `brew install binaryen`
        - For Linux / Unix, download the latest version from [the Github release page](https://github.com/WebAssembly/binaryen/releases) and put it under your `$PATH`
        - **Note**: Linux package managers may download legacy binaryen. We strongly suggest installing the latest binary from the Github release page listed above.
    - Install dylint-link toolchain: `cargo install cargo-dylint dylint-link`
    - Install contract toolchain: `cargo install cargo-contract --force`
    - For macOS M1 chip users: `rustup component add rust-src --toolchain nightly-aarch64-apple-darwin`
- Install the frontend toolchain (if you want to hack the frontend as well)
    - Node.js (>=v16), follow the [official tutorial](https://nodejs.org/en/download/package-manager/)
    - Yarn (v1): `npm install --global yarn`

Check your installation with

```bash
$ rustup toolchain list
# stable-x86_64-unknown-linux-gnu (default)
# nightly-x86_64-unknown-linux-gnu

$ cargo --version
# cargo 1.58.0 (f01b232bc 2022-01-19)

$ cargo contract --version
# cargo-contract 0.17.0-unknown-x86_64-linux-gnu

$ node --version
# v17.5.0

$ yarn --version
# 1.22.17
```

### Compile a contract

Clone and initialize the workshop git repo:

```bash
git clone https://github.com/Phala-Network/oracle-workshop.git
cd oracle-workshop
git submodule update --init
```

Build the `EasyOracle` contract:

```bash
cd easy_oracle
cargo contract build
```

A successful run should output a similar log in the console:

```
Original wasm size: 83.2K, Optimized: 43.9K

The contract was built in DEBUG mode.

Your contract artifacts are ready. You can find them in:
/home/workshop/oracle-workshop/target/ink/easy_oracle

  - easy_oracle.contract (code + metadata)
  - easy_oracle.wasm (the contract's code)
  - metadata.json (the contract's metadata)
```

Once the contract is built, you can find the contract artifacts under `target/ink/easy_oracle`. It will produce three files:

- `easy_oracle.wasm`: The wasm binary
- `metadata.json`: The generated ABI file, useful to work with the js-sdk
- `easy_oracle.contract`: The JSON bundle with the content of the above two files, useful to work with Fat Contract UI

There are three contracts in the workshop repo. In this workshop, you only need to work with `EasyOracle`. However, if you want to build the other two contracts, you need to `cd` to their directory and run `cargo contract build` separately.

### Hack the contract

The `EasyOracle` is ready to hack. A simple idea is to change it from verifying the Github account to a Twitter account, where you can use Twitter's [lookup api](https://developer.twitter.com/apitools/api?endpoint=%2F2%2Ftweets%2F%7Bid%7D&method=get) to get a JSON response of the tweet content and the author username.

> Note that Twitter API requires authentication. You can [generate a bearer token](https://developer.twitter.com/en/docs/authentication/oauth-2-0) and seal it in the contract.

> You may also want to use a JSON deserializer in your Fat Contract. However due to [some limitation of ink!](https://substrate.stackexchange.com/a/3325/544), you may want to use [serde-json-core](https://docs.rs/serde-json-core/latest/serde_json_core/) to bypass the float point problem.

To pass the Advanced Challenge, you need to make sure:

- The contract implements the`SubmittableOracle` trait (already done in `EasyOracle`)
- The contract returns the owner account in method `admin()`
- The contract returns the attestation verifier in method `verifier()`
- The contract can generate a valid attestation in method `attest()`

#### Running tests

Once you started to hack, unit tests are your best friend to test your contract. Running a unit test is a little bit different from ink! in this workshop:

```bash
cargo test --features mockable
```

When you are in trouble, consider enabling stack backtrace by tweaking the env var:

```bash
RUST_BACKTRACE=1 cargo test --features mockable
```

And sometimes when you want to use Fat Contract's logger in the unit test:

```bash
RUST_BACKTRACE=1 cargo test --features mockable -- --nocapture
```

#### Test the HTTP requests

Fat Contract supports HTTP requests, but it may not be a good idea to trigger a request in a unit test. It's suggested to mock the response in a unit test like below. Then all the requests in the contract will get the mock response from your function.

```rust
mock::mock_http_request(|_| {
    HttpResponse::ok(b"This gist is owned by address: 0x0101010101010101010101010101010101010101010101010101010101010101".to_vec())
});
```

#### Openbrush library and "trait_definition"

Openbrush is the "OpenZeppelin" in ink! ecosystem. It has a lot of useful utilities and macros to help you build the contract. Openbrush is used in `EasyOracle` and some other contracts to facilitate the cross-contract call and unit tests.

> TODO: Need a comprehensive explanation.

`trait_definition` is a powerful tool to define a common interface for cross-contract invocation in ink!. For now, you can check the following topics:

- [[trait_definition] in the Official Docs](https://ink.substrate.io/basics/trait-definitions)
- [Openbrush](https://github.com/Supercolony-net/openbrush-contracts/)
- [Discussion about cross-contract call in unit tests](https://github.com/Supercolony-net/openbrush-contracts/pull/136)

### Deploy and configure the contract

Once you have finished the test and want to run it in the real testnet, you can start to deploy the contract.

First, compile the contract by `cargo contract build`. Then you can save the `.contract` file for deployment.

Open the [Contracts UI](https://phat.phala.network/). It will show a popup to connect to Polkdot.js extension. Please accept the request, and connect to your wallet. In the connect popup, make sure to connect to the testnet RPC endpoint, and select an account with some test token:

```
wss://poc5.phala.network/ws
```

![](https://i.imgur.com/giRp7Wj.png)

Now you can drag-n-drop your `.contract` file to the upload area. Please leave the _Cluster ID_ the default value. 

![](https://i.imgur.com/hVPNLDm.png)

When the contract is loaded, it will show the constructor selector. If you haven't especially changed it, just use the default constructor (`new`).

![](https://i.imgur.com/QWAS09a.png)

Then click _Submit_. The Contract UI will upload your contract to the blockchain and instantiate it. The process may take half a minute to complete. Once it's ready, go back to the homepage, and your contract will show up.

![](https://i.imgur.com/HIMYIlY.png)

Click on your contract to enter the contract console page. You will see the important contract information like the **CONTRACT ID** (the address of your contract instance).

![](https://i.imgur.com/Vae03il.png)

In the body of the page, you can interact with the contract transaction and query methods.

To invoke a transaction method (with a `TX` tag), you are going to submit a transaction with your wallet, but you don't know the outcome of the transaction. Usually, you also need to query the contract (with a `QUERY` tag) to check its status. The query response will show in the output panel as shown below.

![](https://i.imgur.com/4nOeFmb.png)

#### Submit your solution

Before the submission, please make sure your contract can meet the submission criteria described in the "Hack the contract" section, and that it's deployed on the public testnet.

Open the [Decoded Workshop Dapp]() and switch to the _Advanced Challenge_ page. Fill in the contract id and a valid argument for your `attest()` method, and click the _Verify_ button. The judger will call the `attest()` method with the given arg in your oracle, and check if your submission meets the criteria.

![](https://i.imgur.com/4qHcvvd.png)

If it turns out your submission passed the verification, congratulations, you will win an Advanced Challenge Winner POAP! Get your code on the FatBadges page, and redeem it!

#### (Optional) Issue badge from your oracle

If you want to enable your oracle to issue POAP like the Easy Challenge, you will need to config your contract in the following steps:

1. Config the `FatBadges` contract (ID: `0x083872054018c5b1890b8a901fc4213a385e3e4df5ddcc71405e4000e4244c6c`)
    - Create a new badge by `tx.new_badge`. The caller will be the owner of the badge.
    - Grant the permission to issue badges to your oracle by `tx.add_issuer`
    - Add enough POAP redeem code to your badge by `tx.add_code`. Not that you will need to give a JSON string array in the arg textbox, because the input type is `Vec<String>`
    - Note that all of the above operations are owner-only
    - To check if your badge is configured correctly, call `query.get_total_badges` and `query.get_badge_info`. Each created badge will have a self-incremental id. Usually your newly created badge id is `get_total_badges() - 1`
2. Config your `EasyOracle` contract
    - Set the badges contract and badge id by `tx.config_issuer`. The badge contract should be that of `FatBadges`. The id should be the one you just created.

A more accurate process is described in the [end-to-end test](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/scripts/js/src/e2e.js#L180-L262).

#### (Optional) Interact with the contract programmatically

For contract interaction in node.js, please check the [end-to-end test](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/scripts/js/src/e2e.js#L180-L262) as an example.

For contract interaction in the browser, please check the [Decoded Workshop DApp source](https://github.com/Phala-Network/js-sdk/blob/6c26c1aef0b6ea6eb85b5d75f1492f120233047c/packages/example/pages/easy-challenge.tsx).

You can find basic usage from the [`@phala/sdk` readme](https://github.com/Phala-Network/js-sdk/tree/decoded-2022/packages/sdk).

### Resources

- Fat Contract UI: <https://phat.phala.network/>
- Testnet Endpoint: `wss://poc5.phala.network/ws`
- FatBadges contract id: `0x083872054018c5b1890b8a901fc4213a385e3e4df5ddcc71405e4000e4244c6c`
- [End-to-end test](https://github.com/Phala-Network/oracle-workshop/blob/3fe330fcdfef8f088896c3fba07c9bc79ccecea5/scripts/js/src/e2e.js#L180-L262)
- [Decoded Workshop DApp source](https://github.com/Phala-Network/js-sdk/blob/6c26c1aef0b6ea6eb85b5d75f1492f120233047c/packages/example/pages/easy-challenge.tsx)
- [`@phala/sdk` readme](https://github.com/Phala-Network/js-sdk/tree/decoded-2022/packages/sdk)

## Troubleshooting

### Failed to compile with edition2021 error

> "ERROR: Error invoking cargo metadata", "feature `edition2021` is required"

Please make sure your rust toolchain is at the latest version by running:

```bash
rustup update
```

### Failed to compile with rustlib error

> error: ".../.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/Cargo.lock" does not exist, unable to build with the standard library

Try to add the `rust-src` component:

```bash
rustup component add rust-src --toolchain nightly
```

### Too old binaryen (wasm-opt)

> ERROR: Your wasm-opt version is 91, but we require a version >= 99

Please uninstall your current `binaryen` and reinstall the latest version from [the official repo](https://github.com/WebAssembly/binaryen/releases).