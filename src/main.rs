use microkv::namespace::NamespaceMicrokv;
use microkv::MicroKV;
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use std::path::PathBuf;

fn should_respond(str: &str) -> bool {
    let contents = str.to_lowercase();

    (contents.contains("luna") || contents.contains("луна"))
        && [
            "?",
            "what",
            "is",
            "are you",
            "tell",
            "say",
            "thought",
            "как",
            "дума",
            "скаж",
            "что",
            "почему",
            "зачем",
            "мнение",
            "мысл",
        ]
        .into_iter()
        .any(|needle| contents.contains(needle))
}

fn pick_word(word: String, vec: &[String]) -> Option<&String> {
    let mut thread_rng = rand::thread_rng();
    let guaranteed_pick = match word.to_lowercase().as_str() {
        "i'm" | "the" | "a" | "i" | "to" | "for" => true,
        _ => false,
    };

    match guaranteed_pick || thread_rng.gen_range(0..10) != 0 {
        false => None,
        _ => vec.choose(&mut thread_rng),
    }
}

fn get_starting_words(db: &NamespaceMicrokv) -> Vec<String> {
    match db.get(String::from("__STARTING_WORDS__")) {
        Ok(Some(word)) => word,
        _ => vec![],
    }
}

fn build_sentence(db: &NamespaceMicrokv, words: &[String], level: u8) -> String {
    if level > 10 {
        return String::from("");
    }

    let mut i = 0;
    let mut sentence: String = String::from("");
    let mut cur_next = words.to_owned();
    let mut last_word = String::from("");

    while let Some(word) = pick_word(last_word, &cur_next) {
        if i >= 48 {
            break;
        }

        last_word = word.to_string();

        sentence.push_str(word);
        sentence.push(' ');

        cur_next = match db.get(word) {
            Ok(Some(nw)) => nw,
            _ => break,
        };

        i += 1;
    }

    sentence = sentence.trim().to_string();

    if sentence.ends_with(',') {
        sentence.push(' ');
        sentence.push_str(&build_sentence(db, &get_starting_words(db), level + 1));
        sentence = sentence.trim().to_string();
    }

    if !sentence.ends_with(&['.', '?', '!']) {
        if sentence.ends_with("what") || sentence.ends_with("why") || sentence.ends_with("who") {
            sentence.push('?');
        } else {
            sentence.push('.');
        }
    }

    if sentence == "." {
        sentence = build_sentence(db, words, level + 1);
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

fn get_vec_or_empty(db: &NamespaceMicrokv, key: String) -> Vec<String> {
    match db.get(key) {
        Ok(Some(words)) => words,
        _ => vec![],
    }
}

fn sanitize_word(word: String) -> String {
    word.replace(&[')', '(', '|', '"'], "")
}

fn sanitize_str(msg: String) -> String {
    msg.replace("@everyone", "@\u{200B}everyone")
        .replace("@here", "@\u{200B}here")
        .replace('*', "\\*")
        .replace('`', "\\`")
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

        let guild_id = msg.guild_id.unwrap().as_u64().to_string();
        let channel_db = self.db.namespace(guild_id);

        if should_respond(&msg.content) {
            let words: Vec<String> = get_starting_words(&channel_db);
            let sentence = build_sentence(&channel_db, &words, 0);

            if sentence.is_empty() || sentence == "." {
                return;
            }

            if let Err(why) = msg.channel_id.send_message(&context.http, |m| {
                m.allowed_mentions(|am| am.empty_parse()).content(sanitize_str(sentence))
            }).await {
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

            let first_word = words.first().unwrap();
            let mut first_words = get_vec_or_empty(&channel_db, String::from("__STARTING_WORDS__"));

            first_words.push(first_word.to_string());
            first_words.dedup();

            if channel_db
                .put(String::from("__STARTING_WORDS__"), &first_words)
                .is_err()
            {
                return;
            }

            for word_pairs in words.windows(2) {
                let word = word_pairs.first().unwrap();
                let next_word = word_pairs.last().unwrap();
                let mut next_words = get_vec_or_empty(&channel_db, word.to_string());

                if next_words.len() >= 64 {
                    next_words.remove(0);
                }

                next_words.push(sanitize_word(next_word.to_string()));

                if channel_db.put(word, &next_words).is_err() {
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
