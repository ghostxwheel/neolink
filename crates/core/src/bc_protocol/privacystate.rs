use super::{BcCamera, Error, Result};
use crate::bc::{model::*, xml::*};

impl BcCamera {
    /// Get the [SleepState] xml which contains the privacy (sleep) mode status of the camera
    pub async fn get_privacystate(&self) -> Result<SleepState> {
        let connection = self.get_connection();
        let msg_num = self.new_message_num();
        let mut sub_get = connection
            .subscribe(MSG_ID_GET_PRIVACY_MODE, msg_num)
            .await?;
        let get = Bc {
            meta: BcMeta {
                msg_id: MSG_ID_GET_PRIVACY_MODE,
                channel_id: self.channel_id,
                msg_num,
                response_code: 0,
                stream_type: 0,
                class: 0x6414,
            },
            body: BcBody::ModernMsg(ModernMsg {
                extension: Some(Extension {
                    channel_id: Some(self.channel_id),
                    ..Default::default()
                }),
                payload: None,
            }),
        };

        sub_get.send(get).await?;
        let msg = sub_get.recv().await?;
        if msg.meta.response_code != 200 {
            return Err(Error::CameraServiceUnavailable {
                id: msg.meta.msg_id,
                code: msg.meta.response_code,
            });
        }

        if let BcBody::ModernMsg(ModernMsg {
            payload:
                Some(BcPayloads::BcXml(BcXml {
                    sleep_state: Some(sleepstate),
                    ..
                })),
            ..
        }) = msg.body
        {
            Ok(sleepstate)
        } else {
            Err(Error::UnintelligibleReply {
                reply: std::sync::Arc::new(Box::new(msg)),
                why: "Expected SleepState xml but it was not recieved",
            })
        }
    }

    /// Set the privacy (sleep) mode using the [SleepState] xml
    pub async fn set_privacystate(&self, mut sleep_state: SleepState) -> Result<()> {
        let connection = self.get_connection();

        let msg_num = self.new_message_num();
        let mut sub_set = connection
            .subscribe(MSG_ID_SET_PRIVACY_MODE, msg_num)
            .await?;

        // operate=2 is what the camera expects when setting; ensure it's set
        // regardless of what a prior get_privacystate() call returned it as.
        sleep_state.operate = Some("2".to_string());
        let get = Bc {
            meta: BcMeta {
                msg_id: MSG_ID_SET_PRIVACY_MODE,
                channel_id: self.channel_id,
                msg_num,
                response_code: 0,
                stream_type: 0,
                class: 0x6414,
            },
            body: BcBody::ModernMsg(ModernMsg {
                extension: Some(Extension {
                    channel_id: Some(self.channel_id),
                    ..Default::default()
                }),
                payload: Some(BcPayloads::BcXml(BcXml {
                    sleep_state: Some(sleep_state),
                    ..Default::default()
                })),
            }),
        };

        sub_set.send(get).await?;
        if let Ok(reply) =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), sub_set.recv()).await
        {
            let msg = reply?;

            if let BcMeta {
                response_code: 200, ..
            } = msg.meta
            {
                Ok(())
            } else {
                Err(Error::UnintelligibleReply {
                    reply: std::sync::Arc::new(Box::new(msg)),
                    why: "The camera did not except the SleepState xml",
                })
            }
        } else {
            // Some cameras seem to just not send a reply on success, so after 500ms we return Ok
            Ok(())
        }
    }

    /// This is a convience function to control the privacy (sleep) mode
    /// True enables privacy mode (camera stops streaming), false disables it
    pub async fn privacy_set(&self, state: bool) -> Result<()> {
        let sleep_state = SleepState {
            version: "1.1".to_string(),
            operate: Some("2".to_string()),
            sleep: match state {
                true => "1".to_string(),
                false => "0".to_string(),
            },
        };
        self.set_privacystate(sleep_state).await?;
        Ok(())
    }
}
