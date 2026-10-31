//! The Lie Detector (report c025): the tested player's test, which this
//! actor shows, times and judges, and the tester's side of it, by the item
//! or a GM's skill.

use super::ClientActor;
use crate::error::RuntimeError;
use crate::message::{ClientHandle, DirectoryMessage, FieldKey, ServerMessage};
use net::captcha::{self, Captcha};
use net::handler::HandlerResult;
use net::handler::WorldAction;
use net::packet::build::world::anti_macro::{
    build_anti_macro_result, build_anti_macro_result_by, AntiMacroResult, Detector,
};
use net::packet::handle::world::lie_detector::{
    failed, fine_received, passed, refusal, spend_detector, TEST_TIME, TRIES,
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
    by: Detector,
    /// Answers still to come, counting the one awaited.
    tries: u8,
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
        let by = test.by.detector();
        self.lie_test = Some(LieTest {
            captcha,
            deadline: Instant::now() + TEST_TIME,
            tester: reply_to.clone(),
            by,
            tries: TRIES,
        });
        reply_to.send(ServerMessage::LieDetected {
            test,
            outcome: Ok(()),
        });
        // A GM's test goes unannounced; the picture comes in its own window.
        if by == Detector::Item {
            let used = AntiMacroResult::Used { name: tester };
            self.send(&mut build_anti_macro_result(&used)?).await?;
        }
        let challenge = AntiMacroResult::Challenge { first: true, jpeg };
        self.send(&mut build_anti_macro_result_by(by, &challenge)?)
            .await
    }

    /// The tester's side: the Lie Detector goes, or the GM hears whom it
    /// tests, once the test began; or the client hears why it didn't.
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
            Err(refused) => {
                HandlerResult::reply(build_anti_macro_result_by(test.by.detector(), &refused)?)
            }
        };
        self.apply_all(result).await
    }

    /// The player typed `typed` back: right passes; anything else shows a
    /// new picture in the time left while a try remains, or fails.
    pub(super) async fn answer_lie_test(&mut self, typed: String) -> Result<(), RuntimeError> {
        let Some(test) = self.lie_test.as_mut() else {
            return Ok(());
        };
        if !test.captcha.accepts(&typed) {
            if test.tries <= 1 {
                return self.fail_lie_test().await;
            }
            test.tries -= 1;
            test.captcha = captcha::generate(&mut thread_rng());
            let jpeg = std::mem::take(&mut test.captcha.jpeg);
            let challenge = AntiMacroResult::Challenge { first: false, jpeg };
            let by = test.by;
            return self
                .send(&mut build_anti_macro_result_by(by, &challenge)?)
                .await;
        }
        let Some(test) = self.lie_test.take() else {
            return Ok(());
        };
        self.lie_tested().await;
        let result = if test.by == Detector::Skill {
            self.judged(&test, true)?;
            HandlerResult::reply(build_anti_macro_result_by(
                Detector::Skill,
                &AntiMacroResult::Passed,
            )?)
        } else {
            passed(&mut self.in_game_mut()?.player)?
        };
        self.apply_all(result).await
    }

    /// Tell the GM who gave `test` whether the player passed it.
    fn judged(&self, test: &LieTest, passed: bool) -> Result<(), RuntimeError> {
        let name = self.in_game()?.player.name().to_string();
        test.tester
            .send(ServerMessage::LieDetectorJudged { name, passed });
        Ok(())
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
    /// tester, or the GM told, and what the player is to be told.
    async fn fine_for_lie_test(
        &mut self,
    ) -> Result<Option<HandlerResult<WorldAction>>, RuntimeError> {
        let Some(test) = self.lie_test.take() else {
            return Ok(None);
        };
        self.lie_tested().await;
        if test.by == Detector::Skill {
            warn!(self.client_id, "Failed a GM's Lie Detector test");
            self.judged(&test, false)?;
            return Ok(Some(HandlerResult::reply(build_anti_macro_result_by(
                Detector::Skill,
                &AntiMacroResult::Failed,
            )?)));
        }
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

    /// The player this GM tested passed or failed, `name`d with the line and
    /// the screenshot the client takes.
    pub(super) async fn lie_detector_judged(
        &mut self,
        name: String,
        passed: bool,
    ) -> Result<(), RuntimeError> {
        if self.game.is_none() || self.in_cash_shop {
            return Ok(());
        }
        let result = if passed {
            AntiMacroResult::Cleared { name }
        } else {
            AntiMacroResult::Suspected { name }
        };
        self.send(&mut build_anti_macro_result_by(Detector::Skill, &result)?)
            .await
    }

    async fn apply_all(&mut self, result: HandlerResult<WorldAction>) -> Result<(), RuntimeError> {
        for action in result.actions {
            self.apply(action).await?;
        }
        Ok(())
    }
}
