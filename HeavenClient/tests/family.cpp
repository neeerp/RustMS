#include "../Character/Family.h"
#include "../Net/Handlers/Helpers/FamilyParser.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

using namespace ms;

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

	// One chart entry.
	Writer& member(int32_t id, int32_t senior, uint8_t level, const std::string& name, bool online = true)
	{
		put(id).put(senior).put<int16_t>(100).put(level).put<uint8_t>(online);
		put<int32_t>(5).put<int32_t>(50).put<int32_t>(2).put<int32_t>(1);
		put<int32_t>(104000000).put<int32_t>(75);
		return string(name);
	}

	Writer& counts(std::vector<std::pair<int32_t, int32_t>> entries)
	{
		put(static_cast<int32_t>(entries.size()));

		for (auto& entry : entries)
			put(entry.first).put(entry.second);

		return *this;
	}

	InPacket packet() const
	{
		return InPacket(bytes.data(), bytes.size());
	}
};

struct Entry
{
	int32_t id;
	int32_t senior;
	uint8_t level;
	const char* name;
};

FamilyChart chart(int32_t viewed, int32_t self, std::vector<Entry> entries, uint16_t max_juniors = 2)
{
	Writer w;
	w.put(viewed).put(static_cast<int32_t>(entries.size()));

	for (auto& entry : entries)
		w.member(entry.id, entry.senior, entry.level, entry.name);

	w.counts({ { -1, static_cast<int32_t>(entries.size()) }, { 0, 0 } }).counts({}).put(max_juniors);

	InPacket recv = w.packet();
	FamilyChart result = FamilyParser::parse_chart(recv, self);
	require(!recv.available(), "the whole chart is read");

	FamilyNode local;
	local.id = self;
	local.level = 70;
	local.job = 110;
	local.name = "Me";
	result.arrange(local);

	return result;
}

std::string name_at(const FamilyChart& c, size_t slot)
{
	const FamilyNode* node = c.at(slot);

	return node ? node->name : "-";
}

void reads_info_and_privileges()
{
	Writer w;
	w.put<int32_t>(30).put<int32_t>(120).put<int32_t>(-4);
	w.put<uint16_t>(1).put<uint16_t>(2).put<uint16_t>(0).put<int32_t>(7);
	w.string("Maple").string("Be kind").counts({ { 1, 2 } });

	InPacket recv = w.packet();
	FamilyInfo info = FamilyParser::parse_info(recv);
	require(!recv.available(), "the whole info is read");
	require(info.rep == 30 && info.total_rep == 120 && info.today == -4, "info reps");
	require(info.juniors == 1 && info.max_juniors == 2 && info.leader_id == 7, "info juniors and leader");
	require(info.name == "Maple" && info.precepts == "Be kind", "info texts");
	require(info.used_today(1) == 2 && info.used_today(0) == 0, "info uses");

	Writer p;
	p.put<int32_t>(2);
	p.put<uint8_t>(1).put<int32_t>(10).put<int32_t>(3).string("Summon").string("Brings one");
	p.put<uint8_t>(3).put<int32_t>(40).put<int32_t>(1).string("EXP").string("More EXP");

	InPacket precv = p.packet();
	auto privileges = FamilyParser::parse_privileges(precv);
	require(!precv.available(), "the whole list is read");
	require(privileges.size() == 2 && privileges[0].name == "Summon" && privileges[1].cost == 40, "privileges");
	require(privileges[0].needs_target() && !privileges[1].needs_target(), "types 0 and 1 need a name");

	Family family;
	require(!family.loaded() && !family.exists(), "nothing before INFO");
	family.set_info(info);
	family.set_privileges(privileges);
	require(family.loaded() && family.exists(), "loaded by INFO");
	require(family.is_leader(7) && !family.is_leader(8), "only the leader edits the precepts");
	require(family.can_use(0), "enough Rep and uses left");
	require(!family.can_use(1), "not enough Rep");

	info.used[0] = 3;
	family.set_info(info);
	require(!family.can_use(0), "no uses left today");
	require(!family.can_use(2), "no such privilege");

	info.name.clear();
	family.set_info(info);
	require(family.loaded() && !family.exists(), "loaded outside a family");
	require(!family.can_use(0), "no entitlements outside a family");
}

void the_leader_heads_the_tree_and_comes_again_as_the_one_viewed()
{
	// What the server sends a leader with one junior.
	FamilyChart own = chart(1, 1, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" }, { 2, 1, 15, "B" } });
	require(own.mine, "the player's own tree");
	require(name_at(own, 0) == "A" && name_at(own, 3) == "A", "the leader at the top and as the owner");
	require(own.at(0) != own.at(3), "two nodes");
	require(name_at(own, 5) == "B" && name_at(own, 6) == "-", "the junior below");
	require(name_at(own, 2) == "-" && name_at(own, 1) == "-", "nobody above");
	require(!own.at(3)->online, "the owner is not marked online");
	require(own.can_add_junior(), "room for a second junior");

	// The junior's tree: the leader is also the senior.
	FamilyChart junior = chart(2, 2, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" }, { 2, 1, 15, "B" } });
	require(name_at(junior, 0) == "A" && name_at(junior, 2) == "A" && name_at(junior, 3) == "B", "the junior's tree");
	require(name_at(junior, 5) == "-", "no juniors");

	// Another's tree: no buttons.
	FamilyChart theirs = chart(2, 1, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" }, { 2, 1, 15, "B" } });
	require(!theirs.mine && !theirs.can_add_junior(), "not the player's tree");
}

void the_first_entry_is_nobodys_senior()
{
	// Without the leader repeated, the junior hangs from nothing.
	FamilyChart c = chart(1, 1, { { 1, 0, 30, "A" }, { 2, 1, 15, "B" }, { 3, 1, 15, "C" } });
	require(name_at(c, 0) == "A", "the leader at the top");
	require(name_at(c, 3) == "Me", "the owner built from the player");
	require(name_at(c, 5) == "-", "nobody links to the first entry");
}

void two_or_fewer_entries_leave_the_top_empty()
{
	FamilyChart pair = chart(1, 1, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" } });
	require(name_at(pair, 0) == "-" && name_at(pair, 3) == "A", "a lone leader");

	FamilyChart none = chart(1, 1, {});
	require(name_at(none, 0) == "-" && name_at(none, 3) == "Me", "no family: the player alone");
	require(none.at(3)->online && none.at(3)->level == 70, "from the player's own stats");
	require(none.can_add_junior(), "may add a junior");

	FamilyChart other = chart(9, 1, {});
	require(other.at(3) == nullptr, "nobody for another's empty tree");
}

void four_generations_around_the_one_viewed()
{
	FamilyChart c = chart(5, 5, {
		{ 1, 0, 120, "L" },
		{ 3, 2, 100, "S2" },
		{ 4, 3, 90, "S1" },
		{ 5, 4, 70, "V" },
		{ 6, 4, 75, "Sib" },
		{ 7, 5, 50, "J1" },
		{ 8, 5, 80, "J2" },
		{ 9, 7, 30, "G1" },
		{ 10, 7, 30, "G2" },
		{ 11, 8, 30, "G3" },
	});

	const char* expected[] = { "L", "S2", "S1", "V", "Sib", "J1", "J2", "G1", "G2", "G3", "-" };

	for (size_t slot = 0; slot < FamilyChart::SLOTS; slot++)
		require(name_at(c, slot) == expected[slot], "each box");

	require(!c.can_add_junior(), "two juniors already");
	require(c.passed_senior(6), "J2 passed V");
	require(!c.passed_senior(5) && !c.passed_senior(4), "only juniors who passed");
	require(c.count(-1) == 10 && c.count(42) == 0, "counts");
}

void parting_names_the_rep_and_the_fee()
{
	// V (70) under S1 (90) under S2 (100); juniors J1 (50) and J2 (80).
	FamilyChart c = chart(5, 5, {
		{ 1, 0, 120, "L" },
		{ 3, 2, 100, "S2" },
		{ 4, 3, 90, "S1" },
		{ 5, 4, 70, "V" },
		{ 7, 5, 50, "J1" },
		{ 8, 5, 80, "J2" },
	});

	// Parting from a junior: 2X for the owner, X for their senior, by the
	// junior's level, and the fee by the levels between.
	auto j1 = c.parting_at(5);
	require(j1 && j1->junior && j1->id == 7, "J1 is a junior");
	require(j1->rep == 1200 && j1->second_rep == 600, "X(50) = 600");
	require(j1->fee == 2520 * 20, "20 levels between");
	require(j1->text().find("1200 Reputation Points for yourself and 600") == 0, "both figures");
	require(j1->text().find("Fee:50400 Mesos") != std::string::npos, "the fee");

	// One who passed the owner costs no fee.
	auto j2 = c.parting_named("J2");
	require(j2 && j2->junior && j2->fee == 0, "no fee upwards");
	require(j2->text().find("Fee") == std::string::npos, "no fee line");

	// Parting from the senior goes by the owner's level.
	auto s1 = c.parting_named("S1");
	require(s1 && !s1->junior, "S1 is the senior");
	require(s1->rep == 2 * 910 && s1->second_rep == 910, "X(70) = 910");
	require(s1->fee == 2520 * 20, "S1 is 20 above");
	require(s1->text().find("for the Senior and") != std::string::npos, "the 2nd Senior too");

	require(!c.parting_named("S2") && !c.parting_named("j1") && !c.parting_at(4), "only the senior and juniors, by exact name");
	require(!c.parting_at(0), "not the leader");

	// A senior who leads has nobody above.
	FamilyChart junior = chart(2, 2, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" }, { 2, 1, 15, "B" } });
	auto a = junior.parting_at(FamilyChart::SENIOR);
	require(a && !a->junior && a->second_rep == 0 && a->rep == 2 * 150, "X(15) = 150");
	require(a->text().find("Reputations for the Senior") == 4, "the one figure");

	FamilyChart theirs = chart(2, 1, { { 1, 0, 30, "A" }, { 1, 0, 30, "A" }, { 2, 1, 15, "B" } });
	require(!theirs.parting_at(FamilyChart::SENIOR), "not from another's tree");
}

int main()
{
	try
	{
		reads_info_and_privileges();
		the_leader_heads_the_tree_and_comes_again_as_the_one_viewed();
		the_first_entry_is_nobodys_senior();
		two_or_fewer_entries_leave_the_top_empty();
		four_generations_around_the_one_viewed();
		parting_names_the_rep_and_the_fee();
	}
	catch (const std::exception& error)
	{
		std::cerr << "family: " << error.what() << "\n";
		return 1;
	}

	std::cout << "family: all passed\n";
	return 0;
}
