#pragma once

#include "Megaphones.h"

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// Maple TV's items, 5075000 to 5075005 (v83 FUN_0048645b types 0x2e to
	// 0x33), and how their dialog's text is sent (report c056).
	namespace MapleTv
	{
		// The dialog's mode: who the message is for.
		enum class Mode : uint8_t
		{
			// Anyone named, or no one (Messenger).
			MESSENGER,
			// No one (Star).
			STAR,
			// Someone, who must be named (Heart).
			HEART
		};

		inline Mode mode(int32_t item_id)
		{
			return static_cast<Mode>(item_id % 10 % 3);
		}

		// The Megassenger kinds also have the ear, and reach the world with it.
		inline bool mega(int32_t item_id)
		{
			return item_id % 10 >= 3;
		}

		// The seconds the line still needs, as the last SEND_TV said (the
		// manager's +0x3cc), for the next sender's question. REMOVE_TV
		// clears it.
		inline int32_t line_wait = 0;

		// The message is the first five rows of the dialog's wrapped text.
		constexpr size_t ROWS = 5;

		using Lines = std::array<std::string, ROWS>;

		inline Lines lines(const std::string& text, const std::vector<size_t>& starts)
		{
			return Megaphones::wrapped_lines<ROWS>(text, starts);
		}

		// What a Messenger or Megassenger sends first: 3 with a receiver,
		// else 1 (asm a0d372).
		inline uint8_t receiver_flag(const std::string& receiver)
		{
			return receiver.empty() ? 1 : 3;
		}

		// Why a message isn't sent: no receiver for a heart (StringPool
		// 0xf78), or text the filter won't take (0x110).
		enum class Refusal : uint8_t
		{
			NONE, NO_RECEIVER, CURSING
		};

		// Checks a message once its dialog has closed, in v83's order (asm
		// a0d94c): a heart needs a receiver, then the text must pass the
		// filter, which stars out its curse words. Empty text is sent.
		template <typename Censor>
		Refusal check(int32_t item_id, const std::string& receiver, Lines& lines, Censor censor)
		{
			if (mode(item_id) == Mode::HEART && receiver.empty())
				return Refusal::NO_RECEIVER;

			std::string joined;

			for (const std::string& line : lines)
				joined += line;

			return Megaphones::censor_rows(lines, joined, censor) ? Refusal::NONE : Refusal::CURSING;
		}

		// Why the server refused a message (recv 343's code, FUN_006373a0),
		// as StringPool 0xf75 to 0xf77 put it. Empty for a code it lacks.
		inline std::string refusal_text(uint8_t code)
		{
			switch (code)
			{
				case 1:
					return "Non-GM character tried to send GM message.";
				case 2:
					return "You've entered the wrong user name.";
				case 3:
					return "The waiting line is longer than an hour.\\nPlease try using it at a later time.";
				default:
					return "";
			}
		}

		// The question asked before a message is sent (StringPool 0xf79),
		// with the seconds the line still needs.
		inline std::string wait_question(int32_t seconds)
		{
			return "This message will be sent to\\nMaple TV in " + std::to_string(seconds) +
				   " seconds.\\nDo you want to proceed?";
		}
	}
}
