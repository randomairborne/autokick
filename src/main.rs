use std::{collections::HashSet, future::IntoFuture, sync::Arc};

use twilight_cache_inmemory::{InMemoryCache, InMemoryCacheBuilder, ResourceType};
use twilight_gateway::{EventTypeFlags, Intents, Shard, ShardId, StreamExt};
use twilight_http::Client;
use twilight_model::{
    gateway::{
        event::Event, payload::{incoming::GuildCreate, outgoing::request_guild_members::RequestGuildMembersBuilder},
        CloseFrame,
    },
    guild::{Permissions, Role},
    id::{
        marker::{GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};

#[macro_use]
extern crate tracing;
#[tokio::main]
async fn main() {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = valk_utils::get_var("DISCORD_TOKEN");
    let intents = Intents::GUILD_MEMBERS | Intents::GUILDS;
    let resource_types = ResourceType::MEMBER | ResourceType::ROLE;
    let event_types = EventTypeFlags::MEMBER_UPDATE
        | EventTypeFlags::MEMBER_ADD
        | EventTypeFlags::MEMBER_REMOVE
        | EventTypeFlags::MEMBER_CHUNK
        | EventTypeFlags::GUILDS;
    let cache = InMemoryCacheBuilder::new()
        .resource_types(resource_types)
        .build();
    let client = Arc::new(Client::new(token.clone()));
    let me = client
        .current_user()
        .await
        .unwrap()
        .model()
        .await
        .unwrap()
        .id;
    let mut state = AppState {
        me,
        client,
        cache,
        kick_with: HashSet::new(),
    };

    let mut shard = Shard::new(ShardId::ONE, token, intents);
    let sender = shard.sender();
    let shutdown_sender = shard.sender();
    tokio::spawn(async move {
        vss::shutdown_signal().await;
        shutdown_sender.close(CloseFrame::NORMAL).ok();
    });
    tracing::info!("created shard");

    while let Some(item) = shard.next_event(event_types).await {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");
            continue;
        };

        tracing::debug!(?event, "event");
        state.cache.update(&event);
        match event {
            Event::GuildCreate(gc) => {
                if let GuildCreate::Available(gc) = *gc {
                    let req = RequestGuildMembersBuilder::new(gc.id).query("", None);
                    sender.command(&req).ok();
                    kickable_roles(&mut state, &gc.roles);
                }
            }
            Event::GuildUpdate(gu) => kickable_roles(&mut state, &gu.roles),
            Event::MemberAdd(ma) => handle_user(&state, ma.guild_id, ma.user.id, &ma.roles).await,
            Event::MemberUpdate(mu) => {
                handle_user(&state, mu.guild_id, mu.user.id, &mu.roles).await;
            }
            Event::MemberChunk(mc) => {
                for member in mc.members {
                    handle_user(&state, mc.guild_id, member.user.id, &member.roles).await;
                }
            }
            _event => {}
        }
    }
}

fn kickable_roles(state: &mut AppState, roles: &[Role]) {
    for role in roles {
        if role.name.to_ascii_lowercase().contains("kick me") {
            state.kick_with.insert(role.id);
        } else {
            state.kick_with.remove(&role.id);
        }
    }
}

async fn handle_user(
    state: &AppState,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
    roles: &[Id<RoleMarker>],
) {
    if !can_kick(state, guild, user) {
        return;
    }
    unsafe_kick_if_kickable(state, guild, user, roles).await;
}

async fn unsafe_kick_if_kickable(
    state: &AppState,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
    roles: &[Id<RoleMarker>],
) {
    if roles.iter().any(|id| state.kick_with.contains(id)) {
        wrap_handle(kick_user(state.client.clone(), guild, user)).await;
    }
}

fn can_kick(state: &AppState, guild: Id<GuildMarker>, user: Id<UserMarker>) -> bool {
    if max_position(state, guild, user) >= max_position(state, guild, state.me) {
        warn!(
            guild = guild.get(),
            user = user.get(),
            "user has role above me in guild"
        );
        return false;
    }
    let can = state
        .cache
        .permissions()
        .root(state.me, guild)
        .is_ok_and(|v| v.contains(Permissions::KICK_MEMBERS));
    if !can {
        warn!(guild = guild.get(), "no kick permissions in guild");
    }
    can
}

fn max_position(state: &AppState, guild: Id<GuildMarker>, user: Id<UserMarker>) -> i64 {
    state
        .cache
        .member_highest_role(guild, user)
        .and_then(|v| state.cache.role(v))
        .map_or(-1, |v| v.position)
}

async fn kick_user(
    client: Arc<Client>,
    guild: Id<GuildMarker>,
    user: Id<UserMarker>,
) -> Result<(), twilight_http::Error> {
    client.remove_guild_member(guild, user).await.map(|_| ())
}

#[allow(clippy::unused_async)]
async fn wrap_handle<F: IntoFuture<Output = Result<(), twilight_http::Error>> + Send + 'static>(
    fut: F,
) where
    <F as IntoFuture>::IntoFuture: Send,
{
    tokio::spawn(async {
        if let Err(source) = fut.await {
            {
                error!(?source, "Error");
            }
        }
    });
}

struct AppState {
    client: Arc<Client>,
    me: Id<UserMarker>,
    cache: InMemoryCache,
    kick_with: HashSet<Id<RoleMarker>>,
}
