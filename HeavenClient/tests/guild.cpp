#include "../Character/Guild.h"
#include "../Net/Handlers/Helpers/GuildParser.h"
#include <array>
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

	template <typename T>
	Writer& put(T value)
	{
		int8_t raw[sizeof(T)];
		std::memcpy(raw, &value, sizeof(T));
		bytes.insert(bytes.end(), raw, raw + sizeof(T));
		return *this;
	}

	Writer& string(const std::string& text)
	{
		put<int16_t>(static_cast<int16_t>(text.size()));
		bytes.insert(bytes.end(), text.begin(), text.end());
		return *this;
	}

	// A GUILDMEMBER record: the name NUL-padded to 13 bytes, then six ints.
	Writer& member(const std::string& name, int32_t rank, int32_t online)
	{
		std::string padded = name;
		padded.resize(13, '\0');
		bytes.insert(bytes.end(), padded.begin(), padded.end());
		return put<int32_t>(100).put<int32_t>(30).put(rank).put(online).put<int32_t>(0).put<int32_t>(5);
	}
};

int main()
{
	try
	{
		using namespace ms;

		// The bytes RustMS's build_guild_info test expects after the mode
		// and the in-guild flag.
		Writer w;
		w.put<int32_t>(4).string("Testers");

		for (const char* title : { "Master", "Jr. Master", "Member", "", "" })
			w.string(title);

		w.put<uint8_t>(2).put<int32_t>(1).put<int32_t>(2);
		w.member("Alice", 1, 1).member("Bobby", 3, 0);
		w.put<int32_t>(10).put<int16_t>(1001).put<uint8_t>(2).put<int16_t>(2003).put<uint8_t>(4);
		w.string("Hi").put<int32_t>(7).put<int32_t>(0);

		InPacket recv(w.bytes.data(), w.bytes.size());
		GuildData data = GuildParser::parse_guild(recv);

		require(!recv.available(), "GUILDDATA is read to its end");
		require(data.id == 4 && data.name == "Testers", "the id and name");
		require(data.titles[1] == "Jr. Master" && data.titles[4].empty(), "five titles");
		require(data.members.size() == 2, "two members");
		require(data.members[0].id == 1 && data.members[0].name == "Alice", "member ids line up with records");
		require(data.members[0].job == 100 && data.members[0].level == 30, "job and level");
		require(data.members[0].online && !data.members[1].online, "online flags");
		require(data.members[1].id == 2 && data.members[1].rank == 3, "the second member's rank");
		require(data.capacity == 10 && data.notice == "Hi" && data.points == 7, "capacity, notice, points");
		require(data.emblem.background == 1001 && data.emblem.background_color == 2, "emblem background");
		require(data.emblem.mark == 2003 && data.emblem.mark_color == 4, "emblem mark");

		Guild guild;
		require(!guild.exists() && guild.online_ids(1).empty(), "a new player has no guild");

		guild.update(data);
		require(guild.exists() && guild.is_master(1) && !guild.is_master(2), "the first member is the master");
		require(guild.title(1) == "Master" && guild.title(3) == "Member", "titles by rank");
		require(guild.title(0).empty() && guild.title(6).empty(), "no title out of range");
		require(guild.online_count() == 1, "one member online");
		require(guild.online_ids(1).empty(), "guild chat skips the player");
		require(guild.online_ids(9) == std::vector<int32_t>{ 1 }, "guild chat goes to online members");
		require(guild.find(2) && guild.find(2)->name == "Bobby" && !guild.find(3), "find by id");

		require(guild.rank(2) == 3 && guild.rank(9) == 0, "ranks, and none for outsiders");
		require(!guild.full(), "two of ten");

		Writer joined;
		joined.member("Carol", 5, 1);
		InPacket record(joined.bytes.data(), joined.bytes.size());
		GuildMember carol = GuildParser::parse_member(record);
		require(!record.available() && carol.name == "Carol" && carol.rank == 5, "one member's record");
		carol.id = 3;
		guild.add(carol);
		guild.add(carol);
		require(guild.get_data().members.size() == 3 && guild.find(3)->online, "a member joins once");
		guild.remove(2);
		require(!guild.find(2) && guild.is_master(1) && guild.get_data().members.size() == 2, "a member leaves");

		require(Guild::may_kick(1, 2) && Guild::may_kick(1, 5) && !Guild::may_kick(1, 1), "the master expels the rest");
		require(!Guild::may_kick(2, 2) && Guild::may_kick(2, 3), "a Jr. Master expels the ranks below");
		require(!Guild::may_kick(3, 4) && !Guild::may_kick(1, 0), "members don't, and not without a pick");

		// Alice (1) is the master and Carol (3) holds rank 5, which has no
		// title; ranks 4 and 5 have none.
		require(guild.lowest_rank() == 3, "rank 3 is the lowest with a title");
		require(guild.rank_held(1) && guild.rank_held(5) && !guild.rank_held(2), "ranks held");

		require(Guild::may_raise(1, 3) && Guild::may_raise(1, 5) && !Guild::may_raise(1, 2), "the master raises up to Jr. Master");
		require(Guild::may_raise(2, 4) && !Guild::may_raise(2, 3), "a Jr. Master raises up to Member");
		require(!Guild::may_raise(3, 5) && !Guild::may_raise(1, 0), "members don't, and not without a pick");
		require(guild.may_lower(1, 2) && !guild.may_lower(1, 3), "no one is lowered from the lowest rank");
		require(!guild.may_lower(2, 2) && !guild.may_lower(1, 1) && !guild.may_lower(3, 4), "nor the master, nor by members");

		std::array<std::string, 5> working = guild.get_data().titles;
		require(Guild::min_title(3) == 4 && Guild::min_title(4) == 0, "ranks 1 to 3 need a title");
		require(guild.check_title(working, 1, "ab") == Guild::Title::TOO_SHORT, "a short title");
		require(guild.check_title(working, 3, "") == Guild::Title::TOO_SHORT, "rank 3 keeps a title");
		require(guild.check_title(working, 2, "Captain") == Guild::Title::OK, "a fine title");
		require(guild.check_title(working, 2, "IllIX") == Guild::Title::NOT_ALLOWED, "four of I and l");
		require(guild.check_title(working, 2, "Big Boss") == Guild::Title::NOT_ALLOWED, "no spaces");
		require(guild.check_title(working, 5, "abc") == Guild::Title::NOT_ALLOWED, "a title takes four letters");
		require(guild.check_title(working, 4, "") == Guild::Title::OK, "an empty rank 4 stays empty");
		require(guild.check_title(working, 5, "") == Guild::Title::HELD, "Carol holds rank 5");

		Guild::set_title(working, 5, "Recruit");
		require(working[3] == "Member" && working[4] == "Recruit", "rank 4 is named when rank 5 is");
		require(guild.check_title(working, 4, "") == Guild::Title::LOWER_RANK, "rank 4 keeps a title under rank 5");
		Guild::set_title(working, 4, "Squire");
		require(working[3] == "Squire", "rank 4 renamed");
		require(guild.titles_differ(working), "the titles changed");

		std::array<std::string, 5> shouted = guild.get_data().titles;
		shouted[0] = "MASTER";
		require(!guild.titles_differ(shouted), "case alone is no change");

		require(guild.set_rank(3, 4) && guild.rank(3) == 4 && !guild.set_rank(9, 2), "a member's rank changes");
		guild.set_titles({ "Master", "Jr. Master", "Member", "Squire", "" });
		require(guild.lowest_rank() == 4, "rank 4 is the lowest while rank 5 has no title");
		require(guild.may_lower(2, 3) && !guild.may_lower(2, 4) && !guild.may_lower(2, 2), "a Jr. Master lowers rank 3, not the lowest");
		require(guild.may_lower(1, 3) && !guild.may_lower(1, 4), "the master lowers rank 3, not the lowest");
		guild.set_titles(working);
		require(guild.title(5) == "Recruit" && guild.lowest_rank() == 5, "new titles");
		guild.set_notice("Hello");
		require(guild.get_data().notice == "Hello", "a new notice");

		guild.clear();
		require(!guild.exists() && guild.get_data().members.empty(), "clearing forgets the guild");
		require(guild.lowest_rank() == 0 && guild.rank_held(3), "no guild, no ranks");

		require(Guild::check_name("Testers") == Guild::Name::OK, "letters are fine");
		require(Guild::check_name("Guild123") == Guild::Name::OK, "digits are fine");
		require(Guild::check_name("abc") == Guild::Name::TOO_SHORT, "three letters are too few");
		require(Guild::check_name("abcdefghijklm") == Guild::Name::NOT_ALLOWED, "thirteen are too many");
		require(Guild::check_name("Bad Name") == Guild::Name::NOT_ALLOWED, "no spaces");
		require(Guild::check_name("IllIX") == Guild::Name::NOT_ALLOWED, "fewer than four of I and l");

		require(!GuildEmblem().shown(), "no emblem without a background or a mark");
		GuildEmblem background_only;
		background_only.background = 1000;
		require(background_only.shown(), "a background alone shows");

		require(std::string(guild_mark_category(2003)) == "Animal", "2xxx marks are animals");
		require(std::string(guild_mark_category(3000)) == "Plant", "3xxx marks are plants");
		require(std::string(guild_mark_category(4021)) == "Pattern", "4xxx marks are patterns");
		require(std::string(guild_mark_category(5035)) == "Letter", "5xxx marks are letters");
		require(std::string(guild_mark_category(9026)) == "Etc", "the rest are Etc");

		std::cout << "Guild passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
