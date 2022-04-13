use std::env;

use microkv::MicroKV;
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    utils::MessageBuilder,
};
use std::path::PathBuf;

fn should_respond(str: &str) -> bool {
    let contents = str.to_lowercase();

    contents.contains("luna")
        && (contents.contains("what")
            || contents.contains("tell")
            || contents.contains("say")
            || contents.contains("thought")
            || contents.contains("opinion"))
}

fn pick_word(vec: &[String]) -> Option<&String> {
    let rn: u64 = rand::thread_rng().gen();

    match rn % 10 {
        0 => None,
        _ => vec.choose(&mut rand::thread_rng()),
    }
}

fn build_sentence(db: &MicroKV, words: &[String]) -> String {
    let mut i = 0;
    let mut sentence = String::from("");
    let mut cur_next = words.to_owned();

    while let Some(word) = pick_word(&cur_next) {
        if i >= 48 {
            break;
        }

        sentence.push_str(word);
        sentence.push(' ');

        cur_next = match db.get(word) {
            Ok(Some(nw)) => nw,
            _ => break,
        };

        i += 1;
    }

    sentence = String::from(sentence.to_string().trim());

    if !sentence.ends_with('.') {
        sentence.push('.');
    }

    if sentence == "." {
        sentence = build_sentence(db, words);
    }

    sentence
}

fn channel_blacklisted(name: &str) -> bool {
    name.contains("staff")
        || name.contains("admin")
        || name.contains("moderator")
        || name.contains("priv")
        || name.contains("appeals")
        || name == "mods"
        || name == "lounge"
}

fn get_vec_or_empty(db: &MicroKV, key: String) -> Vec<String> {
    match db.get(key) {
        Ok(Some(words)) => words,
        _ => vec![],
    }
}

struct Handler {
    db: MicroKV,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if should_respond(&msg.content) {
            let word: Vec<String> = match self.db.get(String::from("__STARTING_WORDS__")) {
                Ok(Some(word)) => word,
                _ => return,
            };

            let sentence = build_sentence(&self.db, &word);
            let response = MessageBuilder::new().push_safe(sentence).build();

            if let Err(why) = msg.channel_id.say(&context.http, &response).await {
                println!("Error sending message: {:?}", why);
            }
        } else {
            let channel = match msg.channel_id.to_channel(&context).await {
                Ok(channel) => channel.guild().unwrap(),
                Err(_) => {
                    return;
                }
            };

            if channel_blacklisted(&channel.name) {
                return;
            }

            let words: Vec<&str> = msg.content.split(' ').collect();

            if words.len() < 2 {
                return;
            }

            println!("Recording words from message: {}", msg.content);

            let first_word = words.first().unwrap();
            let mut first_words = get_vec_or_empty(&self.db, String::from("__STARTING_WORDS__"));

            first_words.push(first_word.to_string());
            first_words.dedup();

            if self
                .db
                .put(String::from("__STARTING_WORDS__"), &first_words)
                .is_err()
            {
                return;
            }

            for word_pairs in words.windows(2) {
                let word = word_pairs.first().unwrap();
                let next_word = word_pairs.last().unwrap();
                let mut next_words = get_vec_or_empty(&self.db, word.to_string());

                if next_words.len() >= 64 {
                    next_words.remove(0);
                }

                next_words.push(next_word.to_string());

                if self.db.put(word, &next_words).is_err() {
                    return;
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let db = MicroKV::open_with_base_path("messages", PathBuf::from("./words.db"))
        .expect("Failed to create MicroKV from a stored file or create MicroKV for this file")
        .set_auto_commit(true);

    let handler = Handler { db };

    let mut client = Client::builder(&token)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}