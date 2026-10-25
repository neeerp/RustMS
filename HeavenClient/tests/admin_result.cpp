#include "../Net/Handlers/Helpers/AdminResult.h"

#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	struct Writer
	{
		std::vector<int8_t> bytes;

		template <typename T>
		Writer& put(T value)
		{
			int8_t raw[sizeof(T)];
			std::memcpy(raw, &value, sizeof(T));
			bytes.insert(bytes.end(), raw, raw + sizeof(T));
			return *this;
		}

		Writer& str(const std::string& text)
		{
			put<int16_t>(static_cast<int16_t>(text.size()));
			bytes.insert(bytes.end(), text.begin(), text.end());
			return *this;
		}

		ms::AdminResult read() const
		{
			ms::InPacket packet(bytes.data(), bytes.size());
			ms::AdminResult result = ms::AdminResult::read(packet);
			require(packet.length() == 0, "the whole packet is read");
			return result;
		}
	};

	using Kind = ms::AdminResult::Kind;
}

int main()
{
	try
	{
		ms::AdminResult sent = Writer().put<uint8_t>(0x1e).put<uint8_t>(1).read();
		require(sent.kind == Kind::LINE && sent.chat == 9, "a warning sent is a type 9 line");
		require(sent.line == "Your warning has been successfully sent.", "and says so");

		ms::AdminResult unsent = Writer().put<uint8_t>(0x1e).put<uint8_t>(0).read();
		require(unsent.line.rfind("Unable to send the message.", 0) == 0, "a warning with no name is refused");

		ms::AdminResult hidden = Writer().put<uint8_t>(0x10).put<uint8_t>(1).read();
		require(hidden.kind == Kind::HIDDEN && hidden.hidden, "hide reads its state");

		ms::AdminResult npc = Writer().put<uint8_t>(0x09).str("Cody").str("quest").str("1").read();
		require(npc.kind == Kind::LINE && npc.chat == 0xc && npc.line == "[Cody:quest] = 1", "an NPC value is shown");

		ms::AdminResult no_npc = Writer().put<uint8_t>(0x09).str("").read();
		require(no_npc.chat == 0xc && no_npc.line == "You have either entered a wrong NPC name or", "an NPC name that isn't found");

		// 0x13 reads 0x1d's byte after its own.
		ms::AdminResult map = Writer().put<uint8_t>(0x13).put<uint8_t>(0).put<int32_t>(910000001).put<uint8_t>(0).read();
		require(map.kind == Kind::MERCHANT_MAP && map.map_id == 910000001, "a merchant found on a map");

		ms::AdminResult channel = Writer().put<uint8_t>(0x13).put<uint8_t>(1).put<uint8_t>(2).put<uint8_t>(1).read();
		require(channel.line == "Hired Merchant located at : Channel 3", "a merchant found on a channel");

		ms::AdminResult missing = Writer().put<uint8_t>(0x13).put<uint8_t>(1).put<uint8_t>(0xFF).put<uint8_t>(0).read();
		require(missing.line == "Unable to find the hired merchant.", "a merchant not found");

		require(Writer().put<uint8_t>(0x1d).put<uint8_t>(0).read().kind == Kind::NONE, "a quiet failure shows nothing");
		ms::AdminResult failed = Writer().put<uint8_t>(0x1d).put<uint8_t>(1).read();
		require(failed.kind == Kind::LINE && failed.line.empty(), "any other byte logs an empty line");

		require(Writer().put<uint8_t>(0x1b).read().kind == Kind::MINIMAP_OFF, "the minimap off");
		require(Writer().put<uint8_t>(0x1c).read().kind == Kind::MINIMAP_ON, "the minimap on");

		ms::AdminResult text = Writer().put<uint8_t>(0x26).str("hello").read();
		require(text.chat == 0xb && text.line == "hello", "0x26 shows its string");

		require(Writer().put<uint8_t>(0x04).put<uint8_t>(0).read().line == "You have successfully blocked access.", "a block");
		require(Writer().put<uint8_t>(0x06).put<uint8_t>(1).read().line == "You have entered an invalid character name.", "a bad name");
		require(Writer().put<uint8_t>(0x07).read().kind == Kind::NONE, "other modes are ignored");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "admin_result tests passed\n";
	return 0;
}
