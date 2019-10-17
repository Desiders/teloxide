use async_trait::async_trait;

use crate::{
    bot::Bot,
    network,
    requests::{Request, ResponseResult},
    types::{ChatId, ChatPermissions, True},
};

/// Use this method to restrict a user in a supergroup. The bot must be an
/// administrator in the supergroup for this to work and must have the
/// appropriate admin rights. Pass True for all permissions to lift restrictions
/// from a user. Returns True on success.
#[derive(Debug, Clone, Serialize)]
pub struct RestrictChatMember<'a> {
    #[serde(skip_serializing)]
    bot: &'a Bot,
    ///Unique identifier for the target chat or username of the target
    /// supergroup (in the format @supergroupusername)
    pub chat_id: ChatId,
    ///Unique identifier of the target user
    pub user_id: i32,
    ///New user permissions
    pub permissions: ChatPermissions,
    ///Date when restrictions will be lifted for the user, unix time. If user
    /// is restricted for more than 366 days or less than 30 seconds from the
    /// current time, they are considered to be restricted forever
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_date: Option<u64>,
}

#[async_trait]
impl Request for RestrictChatMember<'_> {
    type Output = True;

    async fn send_boxed(self) -> ResponseResult<Self::Output> {
        self.send().await
    }
}

impl RestrictChatMember<'_> {
    async fn send(self) -> ResponseResult<True> {
        network::request_json(
            self.bot.client(),
            self.bot.token(),
            "restrictChatMember",
            &self,
        )
        .await
    }
}

impl<'a> RestrictChatMember<'a> {
    pub(crate) fn new<C, U, P>(
        bot: &'a Bot,
        chat_id: C,
        user_id: U,
        permissions: P,
    ) -> Self
    where
        C: Into<ChatId>,
        U: Into<i32>,
        P: Into<ChatPermissions>,
    {
        Self {
            bot,
            chat_id: chat_id.into(),
            user_id: user_id.into(),
            permissions: permissions.into(),
            until_date: None,
        }
    }

    pub fn chat_id<C>(mut self, value: C) -> Self
    where
        C: Into<ChatId>,
    {
        self.chat_id = value.into();
        self
    }

    pub fn user_id<U>(mut self, value: U) -> Self
    where
        U: Into<i32>,
    {
        self.user_id = value.into();
        self
    }

    pub fn permissions<P>(mut self, value: P) -> Self
    where
        P: Into<ChatPermissions>,
    {
        self.permissions = value.into();
        self
    }

    pub fn until_date<T>(mut self, value: T) -> Self
    where
        T: Into<u64>,
    {
        self.until_date = Some(value.into());
        self
    }
}
