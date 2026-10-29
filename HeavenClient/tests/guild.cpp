#include "../Character/Guild.h"
#include "../Character/GuildEmblemRules.h"
#include "../Character/GuildSort.h"
#include "../Net/Handlers/Helpers/AllianceNews.h"
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
	Writer& member(const std::string& name, int32_t rank, int32_t online, int32_t alliance_rank = 5)
	{
		std::string padded = name;
		padded.resize(13, '\0');
		bytes.insert(bytes.end(), padded.begin(), padded.end());
		return put<int32_t>(100).put<int32_t>(30).put(rank).put(online).put<int32_t>(0).put(alliance_rank);
	}

	struct Member
	{
		int32_t id;
		const char* name;
		int32_t online;
		int32_t alliance_rank;
	};

	// A GUILDDATA with no emblem; the first member is the master.
	Writer& guild(int32_t id, const std::string& name, int32_t alliance_id, std::vector<Member> members)
	{
		put(id).string(name);

		for (const char* title : { "Master", "Jr. Master", "Member", "", "" })
			string(title);

		put<uint8_t>(static_cast<uint8_t>(members.size()));

		for (auto& m : members)
			put(m.id);

		for (size_t i = 0; i < members.size(); i++)
			member(members[i].name, i == 0 ? 1 : 3, members[i].online, members[i].alliance_rank);

		return put<int32_t>(10).put<int16_t>(0).put<uint8_t>(0).put<int16_t>(0).put<uint8_t>(0)
			.string("").put<int32_t>(0).put(alliance_id);
	}

	// The ALLIANCE struct: the guild ids, then the capacity.
	Writer& alliance(int32_t id, const std::vector<int32_t>& guilds, int32_t capacity, const std::string& notice)
	{
		put(id).string("Union");

		for (const char* title : { "Chief", "Lords", "Elder", "", "" })
			string(title);

		put<uint8_t>(static_cast<uint8_t>(guilds.size()));

		for (int32_t guild : guilds)
			put(guild);

		return put(capacity).string(notice);
	}
};

// Applies one ALLIANCE_OPERATION packet for the player, character 1.
ms::AllianceNews::Outcome news(Writer& w, ms::Alliance& alliance, ms::Guild& own, bool whole = true)
{
	ms::InPacket recv(w.bytes.data(), w.bytes.size());
	uint8_t mode = static_cast<uint8_t>(recv.read_byte());
	ms::AllianceNews::Outcome outcome = ms::AllianceNews::apply(mode, recv, alliance, own, 1);

	if (whole)
		require(!recv.available(), "the packet is read to its end");

	return outcome;
}

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
		guild.set_points(120);
		require(guild.get_data().points == 120, "new guild points");
		require(Guild::quest_waitlist_line(0, "Scania-2").empty(), "0 only clears the waitlist");
		require(Guild::quest_waitlist_line(1, "Scania-2") == "Please go see the Guild Quest NPC at Channel Scania-2 immediately to enter.", "the guild's turn");
		require(Guild::quest_waitlist_line(2, "Scania-2") == "Your guild is up next. Please head to the Guild Quest map at Channel Scania-2 and wait.", "next in line");
		require(Guild::quest_waitlist_line(4, "Scania-2") == "There's currently 1 guild participating in the Guild Quest, and your guild is number 3 on the waitlist.", "third in line");
		require(guild.set_online(3, false) && guild.online_count() == 1 && guild.online_ids(9) == std::vector<int32_t>{ 1 }, "a member logs out");
		require(guild.set_online(3, true) && guild.online_count() == 2 && !guild.set_online(9, true), "a member logs in");
		require(guild.set_level_job(3, 31, 110) && guild.find(3)->level == 31 && guild.find(3)->job == 110, "a member's level and job");
		require(!guild.set_level_job(9, 31, 110), "no one else's");
		guild.set_capacity(2);
		require(guild.full() && guild.get_data().capacity == 2, "two members fill a guild of two");
		guild.set_capacity(15);
		require(!guild.full() && guild.get_data().capacity == 15, "a grown guild has room");
		guild.set_emblem(GuildEmblem{ 1005, 3, 2004, 7 });
		require(guild.get_data().emblem == (GuildEmblem{ 1005, 3, 2004, 7 }), "a new emblem");

		guild.clear();
		require(!guild.exists() && guild.get_data().members.empty(), "clearing forgets the guild");
		require(!guild.is_master(1), "no one is master without a guild");
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

		// Lea's emblem maker starts from the guild's emblem if it has a mark, else from the first of all.
		require(emblem::initial(GuildEmblem{ 1005, 3, 3002, 9 }) == (GuildEmblem{ 1005, 3, 3002, 9 }), "the guild's emblem");
		require(emblem::initial(GuildEmblem()) == (GuildEmblem{ 1000, 1, 2000, 1 }), "no emblem, the first");
		require(emblem::initial(background_only) == (GuildEmblem{ 1000, 1, 2000, 1 }), "no mark, the first of all");

		require(emblem::last_id({ "00001003", "00001030", "00001000" }) == 1030, "the last id, in any order");
		require(emblem::last_id({ "00002000", "name", "00002020" }) == 2020, "names that aren't ids don't count");
		require(emblem::last_id({}) == 0, "no ids");

		emblem::LastIds last = { 0, 1030, 2020, 3006, 4021, 5035, 0, 0, 0, 9026 };
		require(emblem::step_background(1000, false, last) == 1030 && emblem::step_background(1030, true, last) == 1000, "backgrounds wrap");
		require(emblem::step_background(1004, true, last) == 1005 && emblem::step_background(1004, false, last) == 1003, "backgrounds step");
		require(emblem::step_mark(2000, false, last) == 2020 && emblem::step_mark(2020, true, last) == 2000, "animals wrap");
		require(emblem::step_mark(9000, false, last) == 9026 && emblem::step_mark(9026, true, last) == 9000, "ETC wraps");
		require(emblem::step_mark(5035, true, last) == 5000 && emblem::step_mark(3003, true, last) == 3004, "marks stay in their category");
		require(emblem::step_color(1, false) == 16 && emblem::step_color(16, true) == 1, "colours wrap");
		require(emblem::step_color(7, true) == 8 && emblem::step_color(7, false) == 6, "colours step");

		require(emblem::category_index(2000) == 0 && emblem::category_index(5035) == 3, "the combo's categories");
		require(emblem::category_index(9026) == 4, "ETC is the last");
		require(emblem::category_index(7000) == 0 && emblem::category_index(0) == 0, "an unknown category is the first");
		require(emblem::category_mark(0) == 2000 && emblem::category_mark(1) == 3000 && emblem::category_mark(2) == 4000, "a category's first mark");
		require(emblem::category_mark(3) == 5000 && emblem::category_mark(4) == 9000, "letters, then ETC");

		// The alliance, as recv 66 tells it (FUN_00a39f4e).
		{
			using Mode = AllianceNews::Mode;
			Guild own;
			Alliance alliance;
			std::vector<Writer::Member> ours = { { 1, "Alice", 1, 3 }, { 2, "Bobby", 1, 3 } };
			std::vector<Writer::Member> theirs = { { 3, "Carol", 0, 2 }, { 4, "Dan", 0, 5 } };
			Writer w;
			w.guild(10, "Tens", 0, ours);
			InPacket recv(w.bytes.data(), w.bytes.size());
			own.update(GuildParser::parse_guild(recv));
			require(own.find(1)->alliance_rank == 3, "GUILDDATA carries the alliance rank");
			require(alliance.online_ids(own, 1).empty(), "no alliance chat outside an alliance");

			ours[0].alliance_rank = 1;
			ours[1].alliance_rank = 5;
			w = Writer();
			w.put<uint8_t>(Mode::INFO).alliance(7, { 10, 20 }, 3, "Hi").guild(10, "Tens", 7, ours).guild(20, "Twenty", 7, theirs);
			news(w, alliance, own);
			require(alliance.exists() && alliance.get_data().name == "Union" && alliance.get_data().capacity == 3, "the alliance");
			require(alliance.get_data().guild_ids == (std::vector<int32_t>{ 10, 20 }) && alliance.get_data().notice == "Hi", "its guilds and notice");
			require(own.get_data().alliance_id == 7 && own.find(1)->alliance_rank == 1, "the player's guild goes to the guild");
			require(alliance.get_guilds().size() == 1 && alliance.find_guild(20) && !alliance.find_guild(10), "the list holds the others");
			require(alliance.title(1) == "Chief" && alliance.title(4).empty() && alliance.title(6).empty(), "titles by rank");
			require(alliance.lowest_rank() == 3, "rank 4 has no title");
			require(alliance.rank_held(own, 2) && !alliance.rank_held(own, 4), "ranks held anywhere in it");
			require(alliance.online_ids(own, 1) == std::vector<int32_t>{ 2 }, "chat goes to those online but the player");

			w = Writer();
			w.put<uint8_t>(Mode::MEMBER_ONLINE).put<int32_t>(7).put<int32_t>(20).put<int32_t>(3).put<uint8_t>(1);
			require(news(w, alliance, own).logged_in == "Carol", "a login elsewhere shows the card");
			require(alliance.online_ids(own, 1) == (std::vector<int32_t>{ 3, 2 }), "the other guilds come first");

			w = Writer();
			w.put<uint8_t>(Mode::MEMBER_ONLINE).put<int32_t>(7).put<int32_t>(10).put<int32_t>(2).put<uint8_t>(1);
			require(news(w, alliance, own).logged_in.empty(), "guildmates aren't looked up");
			w = Writer();
			w.put<uint8_t>(Mode::MEMBER_ONLINE).put<int32_t>(7).put<int32_t>(20).put<int32_t>(1).put<uint8_t>(1);
			require(news(w, alliance, own).logged_in.empty(), "nor the player");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_JOINED).alliance(7, { 10, 20, 30 }, 3, "Hi").put<int32_t>(30)
				.guild(30, "Thirty", 7, { { 5, "Eve", 1, 2 } });
			AllianceNews::Outcome joined = news(w, alliance, own);
			require(joined.chat == "[Thirty] guild joined in your guild alliance." && !joined.ask_info, "another guild joins");
			require(alliance.get_guilds().size() == 2 && alliance.get_data().guild_ids.size() == 3, "and is listed");

			w = Writer();
			w.put<uint8_t>(Mode::MEMBER_LEVEL_JOB).put<int32_t>(7).put<int32_t>(20).put<int32_t>(4).put<int32_t>(70).put<int32_t>(110);
			news(w, alliance, own);
			require(alliance.find_guild(20)->members[1].level == 70 && alliance.find_guild(20)->members[1].job == 110, "level and job");

			w = Writer();
			w.put<uint8_t>(Mode::MASTER).put<int32_t>(7).put<int32_t>(1).put<int32_t>(3);
			news(w, alliance, own);
			require(own.find(1)->alliance_rank == 2 && alliance.find_guild(20)->members[0].alliance_rank == 1, "a new master");

			w = Writer();
			w.put<uint8_t>(Mode::RANK).put<int32_t>(2).put<uint8_t>(4);
			news(w, alliance, own);
			w = Writer();
			w.put<uint8_t>(Mode::RANK).put<int32_t>(4).put<uint8_t>(3);
			news(w, alliance, own);
			require(own.find(2)->alliance_rank == 4 && alliance.find_guild(20)->members[1].alliance_rank == 3, "ranks, here or there");

			w = Writer();
			w.put<uint8_t>(Mode::TITLES).put<int32_t>(7);
			for (const char* title : { "A1", "B2", "C3", "D4", "E5" })
				w.string(title);
			news(w, alliance, own);
			require(alliance.title(5) == "E5" && alliance.lowest_rank() == 5, "titles");

			w = Writer();
			w.put<uint8_t>(Mode::NOTICE).put<int32_t>(8).string("Other");
			news(w, alliance, own);
			require(alliance.get_data().notice == "Hi", "another alliance's notice is ignored");
			w = Writer();
			w.put<uint8_t>(Mode::NOTICE).put<int32_t>(7).string("Meet");
			news(w, alliance, own);
			require(alliance.get_data().notice == "Meet", "the notice");

			w = Writer();
			w.put<uint8_t>(Mode::CHANGED).alliance(7, { 10, 20, 30 }, 4, "Meet");
			news(w, alliance, own);
			require(alliance.get_data().capacity == 4 && alliance.get_data().titles[4].empty(), "the whole alliance anew");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_LEFT).alliance(7, { 10, 20 }, 4, "Meet").put<int32_t>(30)
				.guild(30, "Thirty", 0, { { 5, "Eve", 1, 3 } }).put<uint8_t>(0);
			require(news(w, alliance, own, false).chat.empty(), "another guild leaving says nothing");
			require(!alliance.find_guild(30) && alliance.get_guilds().size() == 1, "and leaves the list");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_CHANGED).put<int32_t>(7).put<int32_t>(20)
				.guild(20, "Twenty", 7, { { 3, "Carol", 1, 1 } });
			news(w, alliance, own);
			require(alliance.find_guild(20)->members.size() == 1, "a guild anew");

			w = Writer();
			w.put<uint8_t>(Mode::DISBANDED).put<int32_t>(7);
			news(w, alliance, own);
			require(!alliance.exists() && alliance.get_guilds().empty(), "disbanded");
			require(own.get_data().alliance_id == 0 && own.find(1)->alliance_rank == 3 && own.find(2)->alliance_rank == 3, "everyone back at rank 3");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_JOINED).alliance(9, { 20, 10 }, 2, "").put<int32_t>(10).guild(10, "Tens", 9, ours);
			joined = news(w, alliance, own);
			require(joined.chat == "You joined [Union] guild alliance." && joined.ask_info, "the player's guild joins");
			require(own.get_data().alliance_id == 9 && alliance.get_guilds().empty(), "the others come with the info");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_LEFT).alliance(8, { 20 }, 2, "").put<int32_t>(10).guild(10, "Tens", 0, ours).put<uint8_t>(1);
			news(w, alliance, own, false);
			require(alliance.exists() && own.get_data().alliance_id == 9, "another alliance's guild left");

			w = Writer();
			w.put<uint8_t>(Mode::GUILD_LEFT).alliance(9, { 20 }, 2, "").put<int32_t>(10).guild(10, "Tens", 0, ours).put<uint8_t>(1);
			require(news(w, alliance, own).chat == "You are dismissed from the guild alliance.", "expelled");
			require(!alliance.exists() && own.get_data().alliance_id == 0, "and out");

			w = Writer();
			w.put<uint8_t>(Mode::ALLIANCE).put<uint8_t>(1).alliance(9, { 10 }, 2, "");
			news(w, alliance, own);
			require(alliance.exists(), "mode 0xc sets the alliance");
			w = Writer();
			w.put<uint8_t>(Mode::ALLIANCE).put<uint8_t>(0);
			news(w, alliance, own);
			require(!alliance.exists(), "or clears it");

			w = Writer();
			w.put<uint8_t>(Mode::GUILDS).put<int32_t>(2).guild(20, "Twenty", 9, theirs).guild(10, "Tens", 9, ours);
			news(w, alliance, own);
			require(alliance.get_guilds().size() == 1 && own.get_data().alliance_id == 9, "mode 0xd stores each guild");

			// The guild's packets bring the alliance along (Alliance::follow).
			// Guilds that came before the player's own guild was known keep
			// it among the others until then.
			Guild early;
			Alliance first;
			AllianceData nine;
			nine.id = 9;
			nine.guild_ids = { 10, 20 };
			first.set(nine);
			first.store(early, GuildData{ 10, "Tens" });
			first.store(early, GuildData{ 20, "Twenty" });
			first.follow(early);
			require(!first.exists() && first.get_guilds().empty(), "no guild, no alliance");
			first.set(nine);
			first.store(early, GuildData{ 10, "Tens" });
			first.store(early, GuildData{ 20, "Twenty" });
			early.update(own.get_data());
			first.follow(early);
			require(first.exists() && first.get_guilds().size() == 1 && !first.find_guild(10), "the player's guild leaves the others");
		}

		{
			// The Alliance tab's rules (FUN_00912b49, FUN_00918204).
			auto guild_data = [](int32_t id, std::string name, std::vector<GuildMember> members)
			{
				GuildData data;
				data.id = id;
				data.name = std::move(name);
				data.members = std::move(members);
				data.alliance_id = 9;

				return data;
			};

			// id, name, job, level, rank, online, alliance rank.
			Guild mine;
			mine.update(guild_data(10, "Tens", {
					{ 1, "Me", 0, 10, 1, true, 1 },
					{ 2, "Bob", 0, 10, 2, true, 3 },
					{ 5, "Eve", 0, 10, 3, false, 4 }
			}));
			Guild outside;
			GuildData lone = guild_data(30, "Lone", { { 7, "Gus", 0, 10, 1, true, 3 } });
			lone.alliance_id = 0;
			outside.update(lone);

			AllianceData nine;
			nine.id = 9;
			nine.guild_ids = { 10, 20 };
			nine.capacity = 3;
			nine.titles = { "Master", "Jr.Master", "Member", "", "" };

			Alliance union_;
			union_.set(nine);
			union_.store(mine, guild_data(20, "Twenty", {
					{ 3, "Carol", 0, 10, 1, true, 2 },
					{ 4, "Dan", 0, 10, 2, true, 3 },
					{ 6, "Fay", 0, 10, 3, false, 5 }
			}));

			require(union_.joined(mine) && !union_.joined(outside), "joined only by a guild in it");
			require(union_.find_member(mine, 1)->name == "Me" && union_.find_member(mine, 3)->name == "Carol" &&
					!union_.find_member(mine, 99), "members of every guild");
			require(union_.guild_of(mine, 2)->id == 10 && union_.guild_of(mine, 4)->id == 20 &&
					!union_.guild_of(mine, 99), "and their guilds");
			require(union_.may_edit_titles(mine, 1) && !union_.may_edit_titles(mine, 2) &&
					!union_.may_edit_titles(outside, 7), "only the alliance's master edits titles");

			Alliance::TabButtons none = union_.tab_buttons(mine, 1, 0, true);
			require(none.invite && !none.withdraw && none.notice && none.info, "the master's own buttons");
			require(!none.kick && !none.change && !none.chat && !none.whisper && !none.party_invite && !none.up &&
					!none.down, "nothing for no one picked");

			Alliance::TabButtons self = union_.tab_buttons(mine, 1, 1, true);
			require(!self.chat && !self.whisper && !self.party_invite, "nor for the player");

			Alliance::TabButtons carol = union_.tab_buttons(mine, 1, 3, true);
			require(carol.kick && carol.change && carol.chat && carol.whisper && carol.party_invite,
					"another guild's master");
			require(!carol.up && !carol.down, "at rank 2, moved by neither button");
			require(!union_.tab_buttons(mine, 1, 3, false).party_invite, "a party invite needs a party leader");

			Alliance::TabButtons dan = union_.tab_buttons(mine, 1, 4, false);
			require(!dan.kick && !dan.change && dan.chat, "not a guild master");
			require(!dan.up && !dan.down, "rank 3 is already the lowest");
			require(!union_.tab_buttons(mine, 1, 5, false).chat, "nor anyone offline");

			Alliance::TabButtons bob = union_.tab_buttons(mine, 2, 4, false);
			require(!bob.invite && !bob.withdraw && !bob.notice && !bob.up && !bob.down && bob.chat,
					"rank 3 moves no one");
			require(!union_.tab_buttons(outside, 7, 0, false).info, "no buttons outside an alliance");

			Guild twenty;
			twenty.update(*union_.find_guild(20));
			Alliance::TabButtons master = union_.tab_buttons(twenty, 3, 0, false);
			require(master.withdraw && master.notice && !master.invite, "a guild master may leave");

			AllianceData full = nine;
			full.capacity = 2;
			full.titles = { "Master", "Jr.Master", "Member", "Four", "" };
			union_.set(full);
			require(!union_.tab_buttons(mine, 1, 0, false).invite, "a full alliance invites no one");
			dan = union_.tab_buttons(mine, 1, 4, false);
			require(!dan.up && dan.down, "down to rank 4 once it has a title");

			std::array<std::string, 5> working = full.titles;
			require(!union_.titles_differ(working), "unchanged titles");
			working[3] = "four";
			require(union_.titles_differ(working), "case counts");

			working = { "Master", "Jr.Master", "Member", "Four", "Five" };
			require(union_.check_title(mine, working, 4, "") == Guild::Title::LOWER_RANK, "rank 4 under rank 5");
			working[4] = "";
			require(union_.check_title(mine, working, 4, "") == Guild::Title::HELD, "Eve holds rank 4");
			require(union_.check_title(mine, working, 5, "") == Guild::Title::HELD, "Fay holds rank 5 in another guild");
			union_.store(mine, guild_data(20, "Twenty", { { 3, "Carol", 0, 10, 1, true, 2 } }));
			require(union_.check_title(mine, working, 5, "") == Guild::Title::OK, "an empty rank may lose its title");
			require(union_.check_title(mine, working, 5, "Fifth") == Guild::Title::OK, "a title");
		}

		// The headings a click sorts by (0x911701), and the sort (0x911fb6).
		{
			using namespace guild_sort;

			require(column_at(1, 1) == NAME && column_at(95, 16) == NAME, "NAME spans 1 to 95");
			require(column_at(96, 8) == JOB && column_at(165, 8) == LEVEL && column_at(279, 8) == TITLE,
					"the other headings");
			require(column_at(0, 8) == NONE && column_at(280, 8) == NONE, "nothing past the edges");
			require(column_at(50, 0) == NONE && column_at(50, 17) == NONE, "nothing above or below");

			Sort order;
			require(order.column == NONE, "the server's order at first");
			order.click(LEVEL);
			require(order.column == LEVEL && order.ascending, "a first click sorts up");
			order.click(LEVEL);
			require(!order.ascending, "the next sorts down");
			order.click(NAME);
			require(order.column == NAME && order.ascending, "another heading sorts up again");

			std::vector<GuildMember> people(4);
			people[0].name = "delta";
			people[0].level = 30;
			people[0].job = 100;
			people[0].rank = 3;
			people[0].alliance_rank = 2;
			people[1].name = "Bravo";
			people[1].level = 50;
			people[1].job = 200;
			people[1].rank = 1;
			people[1].alliance_rank = 4;
			people[2].name = "charlie";
			people[2].level = 30;
			people[2].job = 100;
			people[2].rank = 3;
			people[2].alliance_rank = 1;
			people[3].name = "alpha";
			people[3].level = 10;
			people[3].job = 0;
			people[3].rank = 2;
			people[3].alliance_rank = 3;

			auto names = [&](Sort sort, bool alliance)
			{
				std::vector<const GuildMember*> members;

				for (const GuildMember& member : people)
					members.push_back(&member);

				guild_sort::sort(members, sort, alliance);
				std::string text;

				for (const GuildMember* member : members)
					text += member->name + " ";

				return text;
			};

			require(names(Sort(), false) == "delta Bravo charlie alpha ", "no sort keeps the order");
			require(names({ NAME, true }, false) == "alpha Bravo charlie delta ", "names ignore case");
			require(names({ NAME, false }, false) == "delta charlie Bravo alpha ", "names turn about");
			require(names({ LEVEL, true }, false) == "alpha charlie delta Bravo ", "levels up, ties by name");
			require(names({ LEVEL, false }, false) == "Bravo charlie delta alpha ", "levels down, ties still by name");
			require(names({ JOB, true }, false) == "alpha charlie delta Bravo ", "jobs by id");
			require(names({ TITLE, true }, false) == "Bravo alpha charlie delta ", "titles by rank");
			require(names({ TITLE, true }, true) == "charlie delta alpha Bravo ", "the alliance's by its rank");
		}

		std::cout << "Guild passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
