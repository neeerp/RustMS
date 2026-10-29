#include "MarriageActions.h"

#include "UI.h"

#include "UITypes/UIChatBar.h"
#include "UITypes/UINotice.h"

#include "../Character/Marriage.h"
#include "../Gameplay/MapleMap/FieldLimit.h"
#include "../Gameplay/Stage.h"
#include "../Net/Packets/MarriagePackets.h"

namespace ms
{
	namespace MarriageActions
	{
		namespace
		{
			// StringPool 0x109f, 0x10ac, 0x1095, 0x1094, 0x10a0 and 0x106f.
			constexpr const char* ETC_FULL = "Your ETC slot is full.\\nPlease remove some items.";
			constexpr const char* MARRIED = "You are already married.";
			constexpr const char* ENGAGED = "You are already engaged.";
			constexpr const char* NAME_PROMPT = "Please enter your partner's name.";
			constexpr const char* WAITING = "Waiting for her response...";
			// StringPool 0x10a1 and 0x114.
			constexpr const char* GUEST_PROMPT = "Please enter the guest name.";
			constexpr const char* NOT_HERE = "You can't use it here in this map.";
			// The guest prompt's shortest name: a shorter one says 0x335
			// and the prompt stays open.
			constexpr size_t GUEST_MIN = 4;
			// A character's name.
			constexpr size_t NAME_LIMIT = 12;

			// A proposal is out and unanswered.
			bool proposing = false;

			void notice(const char* text)
			{
				UI::get().emplace<UIOk>(text, [](bool) {});
			}

			void ask(const std::string& name, int32_t item_id)
			{
				if (name.empty())
					return;

				proposing = true;
				ProposePacket(name, item_id).dispatch();

				// Closing it withdraws the proposal; an answer replaces it.
				UI::get().emplace<UIOk>(WAITING, [](bool)
				{
					if (!proposing)
						return;

					proposing = false;
					CancelProposalPacket().dispatch();
				});
			}
		}

		void propose(int32_t item_id)
		{
			Player& player = Stage::get().get_player();

			if (!player.get_inventory().find_free_slot(InventoryType::Id::ETC))
				return notice(ETC_FULL);

			// Only men propose; v83 ignores a woman's box (0xa0fcbb).
			if (player.get_stats().get_female())
				return;

			if (auto& record = player.get_marriage())
				return notice(record->married() ? MARRIED : ENGAGED);

			// The waiting dialog is one of these, so a second box waits too.
			if (proposing || UI::get().dialog_open())
				return;

			UI::get().emplace<UIEnterText>(NAME_PROMPT, [item_id](const std::string& name)
			{
				ask(name, item_id);
			}, NAME_LIMIT);
		}

		void answered()
		{
			proposing = false;
		}

		void invite(int16_t slot)
		{
			Player& player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::USE_WAIT) || UI::get().dialog_open())
				return;

			// No name refuses only a couple that isn't engaged.
			if (marriage::invitation_refusal(player.get_marriage(), "") != nullptr)
				return;

			UI::get().emplace<UIEnterText>(GUEST_PROMPT, [slot](const std::string& name)
			{
				Player& player = Stage::get().get_player();
				auto& record = player.get_marriage();

				if (const char* refusal = marriage::invitation_refusal(record, name))
				{
					if (*refusal)
						notice(refusal);

					return;
				}

				// The server's answer, whatever it is, ends the wait.
				player.request(InviteGuestPacket(name, record->id, slot), ExclRequest::USE_WAIT);
			}, NAME_LIMIT, false, GUEST_MIN);
		}

		void open_invitation(int16_t slot, int32_t item_id)
		{
			if (!Stage::get().get_player().can_request(ExclRequest::USE_WAIT))
				return;

			if (Stage::get().get_mapinfo().get_fieldlimit() & field_limit::INVITATIONS)
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(NOT_HERE, UIChatBar::LineType::SKILL);

				return;
			}

			// Nothing waits on the answer.
			OpenInvitationPacket(slot, item_id).dispatch();
		}
	}
}
