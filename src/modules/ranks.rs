use chrono::Utc;
use rank_card::generate_rank_card;
use serenity::{
    cache::FromStrAndCache, 
    framework::standard::{
        Args, 
        CommandResult, 
        macros::{command, group}
    }, 
    model::prelude::*, prelude::*
};
use rand::Rng;

use crate::DATABASE;
use super::help;

mod rank_card;

pub fn get_level_xp(level: i32) -> i32 {
    5 * level.pow(2) + 50 * level + 100
}

#[group]
#[commands(rank)]
struct Levels;

//TODO allow getting user by tag or username or nickname
#[command]
async fn rank(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    // oh go d
    let target = if args.is_empty() {
        Some(msg.author.clone())
    } else if args.len() == 1 {
        if msg.mentions.is_empty() {
            let arg = args.current().unwrap();
            let user_id = UserId::from_str(&ctx.cache, arg).await;
            if let Ok(user_id) = user_id {
                let guild = msg.guild(&ctx.cache).await.unwrap();
                if let Ok(member) = guild.member(&ctx.http, user_id).await {
                    Some(member.user)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(msg.mentions[0].clone())
        }
    } else {
        None
    };

    
    let target = if let Some(target) = target {
        target
    } else {
        help::send_usage(ctx, msg, "Invalid arguments", "rank [user]").await;
        return Ok(());
    };

    //Don't let target be bot
    if target.bot {
        help::send_error(ctx, msg, "Sorry, you can't use this command on a bot").await;
        return Ok(());
    };

    let guild_id = if msg.guild_id.is_none() {return Ok(());} else {msg.guild_id.unwrap()};

    let role_id = RoleId::from_str(&ctx.cache, "827946575136948226").await;
    let role_id = if role_id.is_err() {return Ok(());} else {role_id.unwrap()};

    let has_tester_role = target.has_role(&ctx.http, guild_id, role_id).await;
    if has_tester_role.is_err() {return Ok(());};
    let has_tester_role = has_tester_role.unwrap();

    if has_tester_role {

        let mut database = DATABASE.get().expect("Database not initialized").lock().await;
        let db_user = database.get_user(guild_id.to_string(), target.id.to_string()).await;

        let writer = generate_rank_card(target, db_user.clone(),
            database.get_rank(guild_id.to_string(), &db_user).await).await;

        msg.channel_id.send_files(&ctx.http, vec![(writer.buffer(), "rank.png")], |m| {m}).await
            .expect("Failed to send message");

    }


    Ok(())
}

pub async fn on_message(ctx: Context, msg: Message) {

    //Don't award points to bots
    if msg.author.bot {return;};

    let guild_id = if msg.guild_id.is_none() {return;} else {msg.guild_id.unwrap()};

    let role_id = RoleId::from_str(&ctx.cache, "827946575136948226").await;
    let role_id = if role_id.is_err() {return;} else {role_id.unwrap()};

    let has_tester_role = msg.author.has_role(&ctx.http, guild_id, role_id).await;
    if has_tester_role.is_err() {return;};
    let has_tester_role = has_tester_role.unwrap();

    if has_tester_role {
        let mut database = DATABASE.get().expect("Database not initialized").lock().await;
        let db_user = database.get_user(guild_id.to_string(), msg.author.id.to_string()).await;
        
        // only award if user hasn't been awarded in the last minute
        if (Utc::now() - db_user.last_xp).num_seconds() > 59 {

            let xp = db_user.xp + rand::thread_rng().gen_range(15..25);
            let level_xp = get_level_xp(db_user.level);

            if xp > level_xp {
                database.set_user_level(guild_id.to_string(), msg.author.id.to_string(),
                    db_user.level+1, xp-level_xp).await;
                level_up(&ctx, &msg, db_user.level+1).await;
            } else {
                database.set_user_xp(guild_id.to_string(), msg.author.id.to_string(),
                    xp).await;
            }
        }

    }

}

async fn level_up(ctx: &Context, msg: &Message, level: i32) {
    msg.channel_id.say(&ctx.http, format!("GG {0}, you just advanced to level {1}!",
        msg.author.mention().to_string(), level))
        .await.expect("Unable to send message");
}