#include "MemoHandlers.h"

#include "../Packets/GameplayPackets.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIMemoNotice.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../Util/Memos.h"

namespace ms
{
	namespace
	{
		enum Mode : int8_t
		{
			LIST = 3,
			SENT = 4,
			NOT_SENT = 5,
			ARRIVED = 7
		};

		const char* const NOT_SENT_TEXTS[] = {
			"The other character is online now.\\nPlease use the whisper function,",
			"Please check the name of the receiving character.",
			"The receiver's inbox is full.\\nPlease try again."
		};

		void notice(const char* message)
		{
			UI::get().emplace<UIOk>(message, [](bool) {});
		}

		void ask()
		{
			AskNotesPacket().dispatch();
		}
	}

	void MemoResultHandler::handle(InPacket& recv) const
	{
		switch (recv.read_byte())
		{
			case LIST:
			{
				std::vector<Memos::Memo> memos(static_cast<uint8_t>(recv.read_byte()));

				for (Memos::Memo& memo : memos)
				{
					memo.id = recv.read_int();
					memo.sender = recv.read_string();
					memo.text = recv.read_string();
					memo.sent = recv.read_long();
					memo.flag = static_cast<uint8_t>(recv.read_byte());
				}

				bool any = !memos.empty();

				if (Memos::Inbox::get().listed(std::move(memos)))
				{
					Memos::Inbox::get().asked();
					ask();
				}

				if (any && !UIMemoNotice::showing())
					UI::get().emplace<UIMemoNotice>();

				break;
			}
			case SENT:
				notice("The note has successfully been sent");
				break;
			case NOT_SENT:
			{
				// The note item may be used again.
				Stage::get().get_player().request_answered();

				uint8_t why = static_cast<uint8_t>(recv.read_byte());

				if (why < std::size(NOT_SENT_TEXTS))
					notice(NOT_SENT_TEXTS[why]);

				break;
			}
			case ARRIVED:
				if (Memos::Inbox::get().arrived())
					ask();

				break;
			default:
				break;
		}
	}
}
