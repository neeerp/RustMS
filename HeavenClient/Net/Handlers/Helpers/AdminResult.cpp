#include "AdminResult.h"

namespace ms
{
	namespace
	{
		AdminResult chat_line(std::string text, uint8_t chat)
		{
			AdminResult result;
			result.kind = AdminResult::Kind::LINE;
			result.line = std::move(text);
			result.chat = chat;
			return result;
		}

		// Mode 0x1d, which 0x13 falls into (0x535903): v83 builds SP 0xab
		// ("Your request failed.") but never shows it, and logs an empty
		// line for any other byte.
		AdminResult request_failed(InPacket& recv, AdminResult shown)
		{
			if (recv.read_byte() != 0 && shown.kind == AdminResult::Kind::NONE)
				return chat_line("", 9);

			return shown;
		}
	}

	AdminResult AdminResult::read(InPacket& recv)
	{
		switch (static_cast<uint8_t>(recv.read_byte()))
		{
			case 0x04:
				recv.read_byte();
				// StringPool 0xa2
				return chat_line("You have successfully blocked access.", 9);
			case 0x05:
				recv.read_byte();
				// StringPool 0xa4
				return chat_line("The unblocking has been successful", 9);
			case 0x06:
				// StringPool 0xa7, 0xa8
				return recv.read_byte() > 0
					? chat_line("You have entered an invalid character name.", 9)
					: chat_line("You have successfully removed the name from the ranks.", 9);
			case 0x09:
			{
				std::string npc = recv.read_string();

				// StringPool 0xaa
				if (npc.empty())
					return chat_line("You have either entered a wrong NPC name or", 0xc);

				std::string key = recv.read_string();
				std::string value = recv.read_string();

				return chat_line("[" + npc + ":" + key + "] = " + value, 0xc);
			}
			case 0x10:
			{
				AdminResult result;
				result.kind = Kind::HIDDEN;
				result.hidden = recv.read_byte() != 0;
				return result;
			}
			case 0x13:
			{
				AdminResult result;

				if (recv.read_byte() == 0)
				{
					// StringPool 0xae
					result.kind = Kind::MERCHANT_MAP;
					result.line = "Hired Merchant located at : ";
					result.map_id = recv.read_int();
					result.chat = 0xc;
				}
				else
				{
					uint8_t channel = static_cast<uint8_t>(recv.read_byte());

					// StringPool 0xae, 0xaf
					result = channel >= 0xFE
						? chat_line("Unable to find the hired merchant.", 0xc)
						: chat_line("Hired Merchant located at : Channel " + std::to_string(channel + 1), 0xc);
				}

				return request_failed(recv, result);
			}
			case 0x1b:
			{
				AdminResult result;
				result.kind = Kind::MINIMAP_OFF;
				return result;
			}
			case 0x1c:
			{
				AdminResult result;
				result.kind = Kind::MINIMAP_ON;
				return result;
			}
			case 0x1d:
				return request_failed(recv, AdminResult());
			case 0x1e:
				// StringPool 0xbb4, 0xbb5
				return recv.read_byte() == 0
					? chat_line("Unable to send the message. Please enter the user's name before warning.", 9)
					: chat_line("Your warning has been successfully sent.", 9);
			case 0x26:
				return chat_line(recv.read_string(), 0xb);
			default:
				return AdminResult();
		}
	}
}
