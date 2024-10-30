# medibot

Bot that reminds you to take your medication. Or of someone else's, naturally. Although don't take _their_ medication.

Anyway.

Under active development unfortunately using Rust and [Teloxide](https://github.com/teloxide/teloxide).

Add your bot token to a `.env` file based on `.env.sample`, run redis from the `compose.yml` file and `cargo run` starts the bot.

Other useful commands are `cargo watch -x test` which runs the tests in watch mode (needs redis). For development `cargo watch -x run` is useful too.
