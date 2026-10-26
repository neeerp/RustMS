#include "../Character/GuildRankRules.h"
#include "../Net/Handlers/Helpers/GuildParser.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

struct Writer
{
	std::vector<int8_t> bytes;

	Writer& put(int32_t value)
	{
		int8_t raw[sizeof value];
		std::memcpy(raw, &value, sizeof value);
		bytes.insert(bytes.end(), raw, raw + sizeof value);
		return *this;
	}

	Writer& string(const std::string& text)
	{
		int16_t length = static_cast<int16_t>(text.size());
		int8_t raw[sizeof length];
		std::memcpy(raw, &length, sizeof length);
		bytes.insert(bytes.end(), raw, raw + sizeof length);
		bytes.insert(bytes.end(), text.begin(), text.end());
		return *this;
	}
};

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::guild_rank;

		// Page p shows ranks 6p-4 to 6p+1 under the leader.
		require(first_rank(1) == 2, "page 1");
		require(first_rank(2) == 8, "page 2");
		require(first_rank(9) == 50, "page 9");

		// BtRight while the next page has a guild on it.
		require(!can_go_right(1, 0), "none");
		require(!can_go_right(1, 7), "seven fit one page");
		require(can_go_right(1, 8), "the eighth is on page 2");
		require(!can_go_right(2, 13), "thirteen fit two pages");
		require(can_go_right(2, 14), "the fourteenth is on page 3");
		require(!can_go_left(1), "first page");
		require(can_go_left(2), "second page");
		require(last_page(0) == 1 && last_page(7) == 1 && last_page(8) == 2 && last_page(50) == 9, "last page");

		require(rank_text(1) == "01", "01");
		require(rank_text(9) == "09", "09");
		require(rank_text(10) == "10", "10");
		require(rank_text(100) == "100", "100");

		require(alpha(1) == 255 && alpha(2) == 180 && alpha(3) == 180 && alpha(4) == 120 && alpha(50) == 120, "alpha");

		GuildEmblem own{ 1001, 3, 2005, 7 };
		require(!has_emblem(GuildEmblem{}), "no emblem");
		require(has_emblem(GuildEmblem{ 0, 0, 0, 1 }), "a colour alone");

		// Zero ids take the viewer's own guild's.
		GuildEmblem row = shown_emblem(GuildEmblem{ 1002, 0, 0, 5 }, own);
		require(row.background == 1002 && row.background_color == 3 && row.mark == 2005 && row.mark_color == 5,
			"fallback");
		GuildEmblem full{ 1003, 1, 3004, 2 };
		require(shown_emblem(full, own) == full, "a full emblem stays");

		// What RustMS's build_guild_ranking writes after the NPC's id: the emblem's mark comes first.
		Writer w;
		w.put(2).string("Alpha").put(2000000000).put(3004).put(5).put(1002).put(7);
		w.string("Beta").put(1999999999).put(0).put(0).put(0).put(0);
		InPacket recv(w.bytes.data(), w.bytes.size());
		std::vector<GuildStanding> guilds = GuildParser::parse_ranking(recv);
		require(!recv.available(), "read to its end");
		require(guilds.size() == 2 && guilds[0].name == "Alpha" && guilds[0].points == 2000000000, "the leader");
		require(guilds[0].emblem == (GuildEmblem{ 1002, 7, 3004, 5 }), "the emblem's order");
		require(guilds[1].name == "Beta" && !has_emblem(guilds[1].emblem), "no emblem");
	}
	catch (const std::exception& e)
	{
		std::cerr << "guild_rank test failed: " << e.what() << '\n';
		return 1;
	}

	std::cout << "guild_rank tests passed\n";
	return 0;
}
