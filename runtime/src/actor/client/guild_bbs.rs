use super::ClientActor;
use crate::{db::spawn_db, error::RuntimeError};
use game::guild_bbs::{cut, icon_item, BODY_MAX_LEN, REPLY_MAX_LEN, TITLE_MAX_LEN};
use net::packet::build::world::guild_bbs::{build_bbs_list, build_bbs_missing, build_bbs_thread};
use net::BbsRequest;

impl ClientActor {
    /// The guild bulletin board. The client asks for the list again after
    /// every change, so only the asker hears back: a written or replied
    /// thread is shown, a reply to a deleted thread is told it is gone, and
    /// nothing is sent for a refused request or a deleted thread. The member's guild and rank are read from the
    /// database.
    pub(super) async fn bbs_request(&mut self, request: BbsRequest) -> Result<(), RuntimeError> {
        if self.in_game()?.player.guild.is_none() {
            return Ok(());
        }
        let by = self.client_id;
        let thread = match request {
            BbsRequest::List { page } => {
                if let Some(page) = spawn_db(move || db::guild_bbs::page(by, page)).await? {
                    self.send(&mut build_bbs_list(&page)?).await?;
                }
                return Ok(());
            }
            BbsRequest::Show { thread } => {
                match spawn_db(move || db::guild_bbs::thread(by, thread)).await? {
                    Some(thread) => self.send(&mut build_bbs_thread(&thread)?).await?,
                    None => self.send(&mut build_bbs_missing()?).await?,
                }
                return Ok(());
            }
            BbsRequest::Write {
                edit,
                notice,
                title,
                body,
                icon,
            } => {
                // The client asks for both before it sends. The database
                // refuses NUL characters.
                if title.is_empty()
                    || body.is_empty()
                    || title.contains('\0')
                    || body.contains('\0')
                {
                    return Ok(());
                }
                let Ok(item) = icon_item(icon) else {
                    return Ok(());
                };
                let inventory = &self.in_game()?.player.inventory;
                if item.is_some_and(|item| inventory.count(item) < 1) {
                    return Ok(());
                }
                let title = cut(&title, TITLE_MAX_LEN).to_owned();
                let body = cut(&body, BODY_MAX_LEN).to_owned();
                let icon = icon as i16;
                spawn_db(move || match edit {
                    Some(id) => db::guild_bbs::edit(by, id, &title, &body, icon),
                    None => db::guild_bbs::post(by, &title, &body, icon, notice),
                })
                .await?
            }
            BbsRequest::Delete { thread } => {
                spawn_db(move || db::guild_bbs::delete(by, thread)).await?;
                return Ok(());
            }
            BbsRequest::Reply { thread, text } => {
                if text.is_empty() || text.contains('\0') {
                    return Ok(());
                }
                let text = cut(&text, REPLY_MAX_LEN).to_owned();
                // The thread was deleted meanwhile.
                let Some(thread) =
                    spawn_db(move || db::guild_bbs::reply(by, thread, &text)).await?
                else {
                    return self.send(&mut build_bbs_missing()?).await;
                };
                Some(thread)
            }
            BbsRequest::DeleteReply { reply } => {
                spawn_db(move || db::guild_bbs::delete_reply(by, reply)).await?
            }
        };
        match thread {
            Some(thread) => self.send(&mut build_bbs_thread(&thread)?).await,
            None => Ok(()),
        }
    }
}
