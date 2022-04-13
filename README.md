# Simple Rust Markov Chain Bot

Should be fairly straightforward to run. Simply:

```
DISCORD_TOKEN=your_bot_token_here cargo run --release
```

Doesn't have any shared library dependencies or anything.

### Design

By design, it reads every message and filters out staff-only channels (determined by channel name), then splits the message contents into words, and creates key-value pairs of `(word, list_of_words_that_may_follow)`. If the list of following words is greater than 64 entries, the first one is popped before adding a new one. The "weight" of each word is determined by how many times it shows up in the list.

It's triggered on every message that contains `Luna` (case-insensitive), followed by any of the following keywords: `what tell say thought opinion`

**Example:**

```
Luna, what do you think?
Luna say something
I think Luna should tell us something
```

### Todos

Maybe I'll make it more sane or something. I dunno. This was a simple Rust exercise designed to be tested in production. ~~Don't test in production though.~~
