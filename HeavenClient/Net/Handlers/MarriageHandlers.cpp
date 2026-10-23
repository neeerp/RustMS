#include "MarriageHandlers.h"

#include "WeddingGifts.h"

#include "../Packets/MarriagePackets.h"

#include "../../Character/Marriage.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/MarriageActions.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIInvitation.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIWeddingRegistry.h"
#include "../../IO/UITypes/UIWishlistInput.h"

namespace ms
{
	void MarriageRequestHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		// The wishlist, which v83 doesn't open over another dialog (0xa29886).
		if (mode == 9)
		{
			if (!UI::get().dialog_open())
				UI::get().emplace<UIWishlistInput>(Stage::get().get_player().get_stats().get_female());

			return;
		}

		if (mode != 0)
			return;

		std::string name = recv.read_string();
		int32_t proposer = recv.read_int();

		// StringPool 0x1089.
		std::string question = name + " has requested engagement.\\nWill you accept this proposal?";

		UI::get().emplace<UIYesNo>(question, [name, proposer](bool accept)
		{
			AnswerProposalPacket(accept, name, proposer).dispatch();
		});
	}

	void WeddingGiftResultHandler::handle(InPacket& recv) const
	{
		enum Mode : int8_t
		{
			GIVE_OPEN = 9,
			RECEIVE_OPEN = 10,
			SENT = 11,
			GIVEN_ALREADY = 12,
			NOT_SENT = 13,
			FAILED = 14,
			TAKEN = 15,
			NOT_TAKEN = 16
		};

		int8_t mode = recv.read_byte();

		if (mode == Mode::GIVE_OPEN || mode == Mode::RECEIVE_OPEN)
		{
			// Neither opens over another dialog.
			if (UI::get().dialog_open())
				return;

			bool give = mode == Mode::GIVE_OPEN;
			UI::get().emplace<UIWeddingRegistry>(give ? UIWeddingRegistry::Kind::GIVE : UIWeddingRegistry::Kind::RECEIVE,
				Stage::get().get_player().get_stats().get_female());

			if (auto window = UI::get().get_element<UIWeddingRegistry>())
			{
				if (give)
					window->show_wishes(wedding_gifts::read_wishlist(recv));
				else
					window->read_gifts(recv);
			}

			return;
		}

		// The rest answer the window that asked.
		auto window = UI::get().get_element<UIWeddingRegistry>();

		if (!window)
			return;

		bool give = window->get_kind() == UIWeddingRegistry::Kind::GIVE;

		if (!window->is_active())
		{
			// A gift given as the window closed is still told.
			if (mode == Mode::SENT && give)
				UI::get().emplace<UIOk>("You have sent a gift.", [](bool) {});

			return;
		}

		switch (mode)
		{
			case Mode::SENT:
				if (give)
				{
					window->show_wishes(wedding_gifts::read_wishlist(recv));
					window->read_gifts(recv);
					window->sent();
				}

				break;
			case Mode::GIVEN_ALREADY:
				// v83 leaves the request pending, so no other gift goes.
				if (give)
					UI::get().emplace<UIOk>("You cannot give more than one present for each wishlist.", [](bool) {});

				break;
			case Mode::NOT_SENT:
			case Mode::FAILED:
				if (give)
				{
					window->answered();
					UI::get().emplace<UIOk>("Failed to send the gift.", [](bool) {});
				}

				break;
			case Mode::TAKEN:
				if (!give)
				{
					window->read_gifts(recv);
					window->taken();
				}

				break;
			case Mode::NOT_TAKEN:
				if (!give)
					window->answered();

				break;
		}
	}

	void MarriageResultHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		MarriageActions::answered();

		if (mode == marriage::RESULT_ENGAGED || mode == marriage::RESULT_MARRIED)
			Stage::get().get_player().set_marriage(MarriageRecord::read(recv));

		if (mode == marriage::RESULT_BROKEN || mode == marriage::RESULT_DIVORCED)
			Stage::get().get_player().set_marriage(std::nullopt);

		if (mode == marriage::RESULT_INVITATION)
		{
			std::string groom = recv.read_string();
			std::string bride = recv.read_string();
			int16_t type = recv.read_short();

			UI::get().emplace<UIInvitation>(groom, bride, type);

			return;
		}

		if (mode == marriage::RESULT_NOTICE)
		{
			if (recv.read_bool())
				UI::get().emplace<UIOk>(recv.read_string(), [](bool) {});

			return;
		}

		if (const char* text = marriage::result_text(mode))
			UI::get().emplace<UIOk>(text, [](bool) {});
	}

	void PartnerMapHandler::handle(InPacket& recv) const
	{
		PartnerSpot spot;
		spot.mapid = recv.read_int();
		spot.id = recv.read_int();

		Stage::get().get_player().set_partner(spot);
	}

	void WeddingProgressHandler::handle(InPacket& recv) const
	{
		uint8_t step = static_cast<uint8_t>(recv.read_byte());
		int32_t groom = recv.read_int();
		int32_t bride = recv.read_int();

		Stage::get().wedding_progress(step, groom, bride);
	}

	void WeddingCeremonyEndHandler::handle(InPacket&) const
	{
		Stage::get().wedding_hearts();
	}
}
