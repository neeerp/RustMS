//! The Lie Detector (report c025): the tested player's test, which this
//! actor shows, times and judges, and the tester's side of it.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{ClientHandle, DirectoryMessage, FieldKey, ServerMessage};
use net::captcha::{self, Captcha};
use net::handler::HandlerResult;
use net::handler::WorldAction;
use net::packet::build::world::anti_macro::{build_anti_macro_result, AntiMacroResult};
use net::packet::handle::world::lie_detector::{
    failed, fine_received, passed, refusal, spend_detector, TEST_TIME,
};
use net::packet::handle::world::LieDetectorUse;
use rand::thread_rng;
use tokio::time::Instant;
use tracing::warn;

/// A test the player is taking.
pub(super) struct LieTest {
    captcha: Captcha,
    /// When the time to answer runs out; the client doesn't end it itself.
    deadline: Instant,
    tester: ClientHandle,
}

impl ClientActor {
    pub(super) fn lie_due(&self) -> Option<Instant> {
        self.lie_test.as_ref().map(|test| test.deadline)
    }

    /// `tester`, on `field`, used a Lie Detector on the player: the test
    /// begins if they are on that field, fighting and not in a test. The
    /// tester hears either way, even when this client's packets fail.
    pub(super) async fn lie_detect(
        &mut self,
        reply_to: ClientHandle,
        test: LieDetectorUse,
        field: FieldKey,
        tester: String,
    ) -> Result<(), RuntimeError> {
        let refused = match self.game.as_ref() {
            Some(game) if !self.in_cash_shop => refusal(
                game.field.key == field,
                self.lie_test.is_some(),
                self.last_attack.map(|at| at.elapsed()),
            ),
            _ => Some(AntiMacroResult::NotFound),
        };
        if let Some(refused) = refused {
            reply_to.send(ServerMessage::LieDetected {
                test,
                outcome: Err(refused),
            });
            return Ok(());
        }
        let mut captcha = captcha::generate(&mut thread_rng());
        let jpeg = std::mem::take(&mut captcha.jpeg);
        self.lie_test = Some(LieTest {
            captcha,
            deadline: Instant::now() + TEST_TIME,
            tester: reply_to.clone(),
        });
        reply_to.send(ServerMessage::LieDetected {
            test,
            outcome: Ok(()),
        });
        let used = AntiMacroResult::Used { name: tester };
        self.send(&mut build_anti_macro_result(&used)?).await?;
        let challenge = AntiMacroResult::Challenge { first: true, jpeg };
        self.send(&mut build_anti_macro_result(&challenge)?).await
    }

    /// The tester's side: the Lie Detector goes once the test began, or the
    /// client hears why it didn't.
    pub(super) async fn lie_detected(
        &mut self,
        test: LieDetectorUse,
        outcome: Result<(), AntiMacroResult>,
    ) -> Result<(), RuntimeError> {
        if self.in_cash_shop {
            return Ok(());
        }
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        let result = match outcome {
            Ok(()) => spend_detector(&mut game.player, &test)?,
            Err(refused) => HandlerResult::reply(build_anti_macro_result(&refused)?),
        };
        self.apply_all(result).await
    }

    /// The player typed `typed` back: right passes, anything else fails.
    pub(super) async fn answer_lie_test(&mut self, typed: String) -> Result<(), RuntimeError> {
        let Some(test) = self.lie_test.as_ref() else {
            return Ok(());
        };
        if !test.captcha.accepts(&typed) {
            return self.fail_lie_test().await;
        }
        self.lie_test = None;
        self.lie_tested().await;
        let result = passed(&mut self.in_game_mut()?.player)?;
        self.apply_all(result).await
    }

    /// A wrong answer, or the time ran out.
    pub(super) async fn fail_lie_test(&mut self) -> Result<(), RuntimeError> {
        if let Some(result) = self.fine_for_lie_test().await? {
            self.apply_all(result).await?;
        }
        Ok(())
    }

    /// The player left in the middle of a test, which fails it; the fine is
    /// saved with them.
    pub(super) async fn leave_lie_test(&mut self) {
        if let Err(error) = self.fine_for_lie_test().await {
            warn!(self.client_id, %error, "Failed to end Lie Detector test");
        }
    }

    /// Fail the test, if there is one: the fine taken and handed to the
    /// tester, and what the player is to be told.
    async fn fine_for_lie_test(
        &mut self,
    ) -> Result<Option<HandlerResult<WorldAction>>, RuntimeError> {
        let Some(test) = self.lie_test.take() else {
            return Ok(None);
        };
        self.lie_tested().await;
        let (result, fine) = failed(&mut self.in_game_mut()?.player)?;
        warn!(self.client_id, fine, "Failed a Lie Detector test");
        test.tester.send(ServerMessage::LieDetectorFine { fine });
        Ok(Some(result))
    }

    async fn lie_tested(&self) {
        let _ = self
            .send_to_directory(DirectoryMessage::LieTested {
                client_id: self.client_id,
            })
            .await;
    }

    /// The player this player tested failed: the fine is theirs, told
    /// unless they are in the Cash Shop.
    pub(super) async fn lie_detector_fine(&mut self, fine: i32) -> Result<(), RuntimeError> {
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        let result = fine_received(&mut game.player, fine)?;
        if self.in_cash_shop {
            return Ok(());
        }
        self.apply_all(result).await
    }

    async fn apply_all(&mut self, result: HandlerResult<WorldAction>) -> Result<(), RuntimeError> {
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}
