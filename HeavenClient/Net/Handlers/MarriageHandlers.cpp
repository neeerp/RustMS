#include "MarriageHandlers.h"

#include "../Packets/MarriagePackets.h"

#include "../../Character/Marriage.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/MarriageActions.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UINotice.h"

namespace ms
{
	void MarriageRequestHandler::handle(InPacket& recv) const
	{
		// Mode 0 is the only one v83 handles here.
		if (recv.read_byte() != 0)
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

	void MarriageResultHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		MarriageActions::answered();

		if (mode == marriage::RESULT_ENGAGED)
			Stage::get().get_player().set_marriage(MarriageRecord::read(recv));

		if (mode == marriage::RESULT_NOTICE)
		{
			if (recv.read_bool())
				UI::get().emplace<UIOk>(recv.read_string(), [](bool) {});

			return;
		}

		if (const char* text = marriage::result_text(mode))
			UI::get().emplace<UIOk>(text, [](bool) {});
	}
}
