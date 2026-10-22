#include "../Character/Pets.h"
#include "../Net/Handlers/Helpers/PetParser.h"
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

	// A pet as the server writes it (write_pet).
	Writer& pet(int32_t itemid, const std::string& name, int64_t serial, int16_t x, int16_t y, uint8_t stance, int16_t fh)
	{
		put(itemid).string(name).put(serial).put(x).put(y).put(stance).put(fh);
		return put<int8_t>(0).put<int8_t>(0);
	}
};

void compacting_list()
{
	using ms::pets::insert;
	using ms::pets::remove;

	std::array<int, 3> slots = {};
	insert(slots, 0, 1);
	insert(slots, 1, 2);
	require(slots == std::array<int, 3>{ 1, 2, 0 }, "pets join at their index");

	insert(slots, 0, 3);
	require(slots == std::array<int, 3>{ 3, 1, 2 }, "a pet put first moves the rest up");

	insert(slots, 1, 4);
	require(slots == std::array<int, 3>{ 3, 4, 1 }, "the last falls off a full list");

	remove(slots, 0);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "the rest close up when one leaves");

	remove(slots, 2);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "an empty slot leaves nothing to close");

	insert(slots, 3, 5);
	remove(slots, 3);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "a fourth index is ignored");
}

void summoning()
{
	using ms::pets::Summon;
	using ms::pets::summon;

	require(summon(false, { 7, 0, 0 }, 8) == Summon::SEND, "without Follow the Lead the server decides");
	require(summon(true, { 0, 0, 0 }, 8) == Summon::SEND, "the first pet comes out");
	require(summon(true, { 7, 0, 0 }, 8) == Summon::ASK_BOSS, "a second asks whether it leads");
	require(summon(true, { 7, 9, 0 }, 9) == Summon::SEND, "a pet that is out goes back");
	require(summon(true, { 7, 9, 6 }, 8) == Summon::FULL, "a fourth is refused");
	require(summon(true, { 7, 9, 6 }, 6) == Summon::SEND, "even with three out, one goes back");

	require(ms::pets::follow_the_lead(0) == 8 && ms::pets::follow_the_lead(412) == 8, "an Explorer's Follow the Lead");
	require(ms::pets::follow_the_lead(1000) == 10000018, "a Noblesse's");
	require(ms::pets::follow_the_lead(2112) == 20000024, "a Legend's");
}

void stances()
{
	using ms::pets::shown;

	require(shown(2) == 0 && shown(3) == 0, "1 moves, either way");
	require(shown(4) == 1 && shown(5) == 1, "2 stands");
	require(shown(6) == 3, "3 jumps");
	require(shown(12) == 4, "6 flies");
	require(shown(22) == 2 && shown(24) == 5, "11 stands otherwise, 12 is hungry");
	require(shown(26) == 6 && shown(28) == 7 && shown(30) == 8, "13 and 14 rest, 15 hangs");
	require(shown(0) == 0 && shown(40) == 0, "the rest move");
}

void spawn_player_pets()
{
	Writer w;
	w.put<int8_t>(1).pet(5000066, "Baby Tiger", 77, 100, -20, 4, 12);
	w.put<int8_t>(1).pet(5000000, "Brown Kitty", 78, 90, -20, 5, 12);
	w.put<int8_t>(1).pet(5000017, "Robo", 79, 80, -20, 2, 12);
	w.put<int8_t>(0).put<int32_t>(0x11223344);

	ms::InPacket recv(w.bytes.data(), w.bytes.size());
	std::vector<ms::PetEntry> pets = ms::PetParser::parse_pets(recv);

	require(pets.size() == 3, "three pets out read as three");
	require(pets[0].itemid == 5000066 && pets[0].name == "Baby Tiger" && pets[0].serial == 77, "the first pet");
	require(pets[0].position == ms::Point<int16_t>(100, -20) && pets[0].stance == 4 && pets[0].fh == 12,
			"where it stands");
	require(pets[2].name == "Robo" && pets[2].serial == 79, "the third pet");
	require(recv.read_int() == 0x11223344, "the 0 after the third ends the list");

	Writer none;
	none.put<int8_t>(0).put<int32_t>(5);
	ms::InPacket empty(none.bytes.data(), none.bytes.size());
	require(ms::PetParser::parse_pets(empty).empty() && empty.read_int() == 5, "no pets is a lone 0");
}

void words()
{
	using ms::pets::split_words;
	using Words = std::vector<std::string>;

	require(split_words("Sleep|Sleepy|Doze|Tired") == Words{ "Sleep", "Sleepy", "Doze", "Tired" }, "split on bars");
	require(split_words(" Hungry? | Food? ") == Words{ "Hungry? ", "Food?" },
			"trimmed in front, and the last at the back too");
	require(split_words("Poo") == Words{ "Poo" }, "a single word");
	require(split_words("a||b") == Words{ "a", "", "b" }, "an empty word is kept between bars");
	require(split_words("  ").empty() && split_words("a|").size() == 1, "nothing after the last bar adds nothing");
	require(ms::pets::trim("\t Tom! \r\n") == "Tom!", "trim strips spaces, tabs and line breaks");
}

void hearing()
{
	using ms::pets::Command;
	using ms::pets::hear;

	std::vector<Command> commands = {
		{ 1, 9, { "Hungry?", "Food?" }, {}, {} },
		{ 10, 30, { "Hungry?", "Food?" }, {}, {} },
		{ 1, 30, { "Sleep", "Doze" }, {}, {} },
	};

	auto heard = hear("  Sleep ", "Baby Tiger", "Baby Tiger", 1, commands);
	require(heard && heard->command == 2 && !heard->named, "a word anywhere in the list, trimmed");
	require(!hear("sleep", "Baby Tiger", "Baby Tiger", 1, commands), "case counts");
	require(!hear("Sleep now", "Baby Tiger", "Baby Tiger", 1, commands), "the whole line must be the word");

	heard = hear("Food?", "Baby Tiger", "Baby Tiger", 12, commands);
	require(heard && heard->command == 1, "the entry for the pet's level");
	require(!hear("Food?", "Baby Tiger", "Baby Tiger", 31, commands), "no entry past its levels");

	heard = hear("Tom! ,~ Doze", "Tom", "Baby Tiger", 5, commands);
	require(heard && heard->command == 2 && heard->named, "a renamed pet called by name");
	heard = hear("TomDoze", "Tom", "Baby Tiger", 5, commands);
	require(heard && heard->named, "no space needed after the name");
	require(!hear("Baby Tiger Doze", "Baby Tiger", "Baby Tiger", 5, commands), "a kind's own name doesn't call it");
	require(!hear("Tom", "Tom", "Baby Tiger", 5, commands), "the name alone isn't a command");
}

void actions()
{
	using ms::pets::action;
	using ms::pets::action_name;
	using ms::pets::reserved;

	std::vector<std::string> customs = { "angry", "chat", "fly", "love" };

	require(action("move", customs) == 0 && action("rest0", customs) == 6 && action("hang", customs) == 8,
			"the base actions");
	require(action("fly", customs) == 4, "a base name wins over the pet's own");
	require(action("angry", customs) == 9 && action("love", customs) == 12, "the pet's own follow");
	require(action("say", customs) == -1 && action("stand", customs) == -1, "one it lacks is -1");
	require(action_name(6, customs) == "rest0" && action_name(10, customs) == "chat", "and back");
	require(action_name(-1, customs).empty() && action_name(13, customs).empty(), "none for none");

	require(reserved("info") && reserved("Interact") && reserved("stand1") && reserved("autoSpeaking"),
			"the image's other children");
	require(!reserved("move") && !reserved("fly") && !reserved("love"), "animations are not");

	require(!ms::pets::idle_due(4992, 8) && ms::pets::idle_due(5000, 8) && !ms::pets::idle_due(5008, 8),
			"once at 5 s");
	require(ms::pets::idle_due(10000, 8), "and every 5 s after");
}

void food()
{
	using ms::pets::closeness_change;
	using ms::pets::Command;
	using ms::pets::food_for;
	using ms::pets::hungriest;

	require(ms::pets::is_food(2120000) && !ms::pets::is_food(2000000) && !ms::pets::is_food(5240000),
			"pet food is 212xxxx");
	require(ms::pets::is_cash_food(5240027) && !ms::pets::is_cash_food(2120000) && !ms::pets::is_cash_food(5000066),
			"cash pet food is 524xxxx");

	require(hungriest({ 60, 40, 80 }) == 1, "the least full eats");
	require(hungriest({ 40, 80, 40 }) == 0, "the first of equals");
	require(hungriest({ -1, 90, -1 }) == 1, "an empty slot is skipped");
	require(hungriest({ 100, -1, -1 }) == 0, "a full pet is still the one fed");
	require(hungriest({ -1, -1, -1 }) == -1, "no pet out, none");

	std::vector<Command> table = {
		{ 1, 9, {}, { { "eat", { "Chomp" } } }, { { "cry", {} } } },
		{ 10, 19, {}, {}, {} },
		{ 1, 30, {}, {}, {} },
	};

	require(food_for(table, 5) == &table[0] && food_for(table, 9) == &table[0], "the entry for its level");
	require(food_for(table, 12) == &table[1], "the next levels' entry");
	require(food_for(table, 25) == &table[2], "the first that holds it");
	require(!food_for(table, 31) && !food_for(table, -1), "none past its levels, or for another's pet");

	require(closeness_change(3, 4) == "The closeness increased (+1)", "closer");
	require(closeness_change(4, 3) == "The closeness decreased (-1)", "further");
	require(closeness_change(4, 4).empty(), "no change, no line");
}

void speech()
{
	using ms::pets::Speech;
	using ms::pets::speech;

	require(speech(14, false, false) == Speech::NONE && speech(14, true, false) == Speech::NONE,
			"below level 15, nothing");
	require(speech(15, false, false) == Speech::SAY && speech(15, false, true) == Speech::SAY,
			"a clean line is said, whatever the pet is doing");
	require(speech(15, true, false) == Speech::SLANG, "a cursing one gives the slang");
	require(speech(30, true, true) == Speech::NONE, "but not while it does an action");

	require(ms::pets::quote_part(0) == 29 && ms::pets::quote_part(1) == 32 && ms::pets::quote_part(2) == 40,
			"the quote rings' parts");
	require(ms::pets::quote_part(3) == -1, "no fourth pet");
}

void naming()
{
	using ms::pets::check_name;
	using ms::pets::Naming;

	require(ms::pets::is_name_tag(5170000) && !ms::pets::is_name_tag(5170001) && !ms::pets::is_name_tag(5240000),
			"the name tag is 5170000");
	require(ms::pets::same_name("Baby Tiger", "baby TIGER") && !ms::pets::same_name("Baby Tiger", "Baby Tige"),
			"names match in any case");

	auto censor = [](const std::string& text)
	{
		std::string starred = text;
		size_t at = starred.find("darn");

		if (at != std::string::npos)
			starred.replace(at, 4, "****");

		return starred;
	};
	auto check = [&](std::string typed, const std::string& current)
	{
		Naming naming = check_name(typed, "Baby Tiger", current, censor);
		return std::make_pair(naming, typed);
	};

	require(check("  Tigger ", "Baby Tiger") == std::make_pair(Naming::OK, std::string("Tigger")), "trimmed");
	require(check(" Tig ", "Baby Tiger").first == Naming::SHORT, "under 4 bytes once trimmed");
	require(check("Tig\x01ger", "Baby Tiger").first == Naming::CURSING, "a control character");
	require(check("darn cat", "Baby Tiger") == std::make_pair(Naming::OK, std::string("**** cat")),
			"curse words starred out");
	require(check("BABY TIGER", "Tigger").first == Naming::TAKEN, "the kind's own name");
	require(check("Tigger", "Tigger").first == Naming::TAKEN, "the name it has");
	require(check("tigger", "Tigger").first == Naming::OK, "its name in another case");
}

void equipment()
{
	using namespace ms::pets;

	require(equip_parts(1802063) == std::array<int16_t, 3>{ 14, 30, 38 }, "clothes");
	require(equip_parts(1802100) == std::array<int16_t, 3>{ 21, 31, 39 }, "the collar goes with the labels");
	require(equip_parts(1822000) == std::array<int16_t, 3>{ 21, 31, 39 }, "label ring");
	require(equip_parts(1832000) == std::array<int16_t, 3>{ 29, 32, 40 }, "quote ring");
	require(equip_parts(1812000) == std::array<int16_t, 3>{ 23, 34, 42 }, "meso magnet");
	require(equip_parts(1812002) == std::array<int16_t, 3>{ 24, 24, 24 }, "the HP pouch is shared");
	require(equip_parts(1812007) == std::array<int16_t, 3>{ 46, 47, 48 }, "ignore pendant");
	require(!equip_parts(1812008), "an unknown ability item");
	require(!equip_parts(1002008), "a cap");

	require(equip_owner(14) == 0 && equip_owner(29) == 0 && equip_owner(46) == 0, "the first pet's parts");
	require(equip_owner(30) == 1 && equip_owner(37) == 1 && equip_owner(47) == 1, "the second pet's parts");
	require(equip_owner(38) == 2 && equip_owner(45) == 2 && equip_owner(48) == 2, "the third pet's parts");
	require(equip_owner(24) == -1 && equip_owner(25) == -1, "the pouches are no one's");
	require(wear_owner(30) == 1 && wear_owner(31) == -1, "clothes by pet");
	require(is_equip_part(14) && is_equip_part(48) && !is_equip_part(15) && !is_equip_part(49), "pet parts");

	Wearer none;
	Wearer tiger{ true, true, true, true, 5 };
	Wearer kitty{ true, false, false, false, 5 };

	require(refuse_equip(14, 0, none) == CANNOT_WEAR, "clothes need a pet");
	require(refuse_equip(30, 0, kitty) == CANNOT_WEAR, "clothes made for another kind");
	require(refuse_equip(38, 0, tiger) == nullptr, "clothes for its kind");
	require(refuse_equip(14, 10, tiger) == STATS_TOO_LOW, "clothes above the pet's level");
	require(refuse_equip(21, 0, none) == NO_PET, "a label needs a pet");
	require(refuse_equip(31, 0, kitty) == CANNOT_WEAR, "a kind without a name tag");
	require(refuse_equip(39, 0, tiger) == nullptr, "a label on a kind with one");
	require(refuse_equip(32, 0, kitty) == CANNOT_WEAR, "a kind without a balloon");
	require(refuse_equip(40, 0, none) == NO_PET, "a quote ring needs a pet");
	require(refuse_equip(47, 0, none) == NO_PET, "the pendant needs a pet");
	require(refuse_equip(23, 0, kitty) == nullptr, "a magnet on any pet");
	require(refuse_equip(35, 0, none) == nullptr, "boots need none");
	require(refuse_equip(24, 0, none) == nullptr, "nor do the pouches");

	using Choices = std::vector<EquipChoice>;
	std::array<Wearer, 3> alone{ kitty, none, none };
	std::array<Wearer, 3> pair{ kitty, tiger, none };
	std::array<Wearer, 3> three{ tiger, kitty, tiger };

	require(equip_choices(1802063, alone) == Choices{ { 0, 14 } }, "one pet out: its part, suited or not");
	require(equip_choices(1822000, { none, none, none }) == Choices{ { 0, 21 } }, "none out: the first pet's part");
	require(equip_choices(1812002, three) == Choices{ { 0, 24 } }, "the pouches don't choose");
	require(equip_choices(1802063, pair) == Choices{ { 1, 30 } }, "clothes for the one pet of their kind");
	require(equip_choices(1802063, three) == Choices{ { 0, 14 }, { 2, 38 } }, "clothes for the pets of their kind");
	require(equip_choices(1812000, three) == Choices{ { 0, 23 }, { 1, 34 }, { 2, 42 } }, "a magnet for any");
	require(equip_choices(1832000, pair) == Choices{ { 0, 29 }, { 1, 32 } }, "the quote ring for any");
	require(equip_choices(1822001, three).empty(), "other rings don't choose");
	require(equip_choices(1002008, three).empty(), "not pet equipment");
}

void info_page()
{
	using namespace ms::pets;
	using Items = std::vector<int32_t>;

	// A magnet and label ring on the first, a pouch on the second, the HP
	// pouch for all, and the third's quote ring.
	std::map<int16_t, int32_t> parts = { { 1, 1002008 }, { 14, 1802000 }, { 21, 1822000 }, { 22, 1812001 },
										 { 24, 1812002 }, { 34, 1812000 }, { 40, 1832000 }, { 47, 1812007 } };
	require(info_items(0, 1802000, parts) == Items{ 1802000, 1822000, 1812001, 1812002 },
			"the first's clothes, then its items by part");
	require(info_items(1, 0, parts) == Items{ 1812002, 1812000, 1812007 }, "the second's");
	require(info_items(2, 0, parts) == Items{ 1812002, 1832000 }, "the third's");
	require(info_items(0, 0, { { 31, 1822001 }, { 23, 1802000 } }).empty(), "other rings and clothes aren't listed");
}

void exception_list()
{
	using namespace ms::pets;
	using Items = std::vector<int32_t>;

	std::map<int32_t, std::string> names = { { 4000000, "Blue Snail Shell" },
											 { 4000019, "Snail Shell" },
											 { 2000000, "Red Potion" },
											 { 4000016, "Red Snail Shell" } };
	auto name = [&](int32_t id) { return names[id]; };

	Items ids = { 4000019, 2000000, IGNORED_MESOS, 4000000, 4000016 };
	sort_ignored(ids, name);
	require(ids == Items{ 4000000, 2000000, 4000016, 4000019, IGNORED_MESOS }, "by name, mesos as \"mesos\"");
	require(ignored_name(IGNORED_MESOS, name) == "mesos", "mesos show trimmed");

	Items cased = { 4000019, 4000000 };
	names[4000000] = "blue snail shell";
	sort_ignored(cased, name);
	require(cased == Items{ 4000019, 4000000 }, "names compare byte by byte");

	names[4000000] = "Blue Snail Shell";
	names[4000001] = " Red Potion ";
	Items listed = { 4000000, 4000019, IGNORED_MESOS };
	require(ignored_slot(listed, 2000000, name) == std::optional<size_t>(1), "added before the next name");
	require(ignored_slot(listed, 4000016, name) == std::optional<size_t>(1), "Red Snail Shell before Snail Shell");
	require(!ignored_slot(Items{ 2000000 }, 4000001, name), "a name already listed stays once");
	require(ignored_slot(Items{ 4000000 }, IGNORED_MESOS, name) == std::optional<size_t>(1), "mesos after capitals");
	require(ignored_slot(Items{}, 4000000, name) == std::optional<size_t>(0), "the first goes first");
	require(!ignored_slot(Items(MAX_IGNORED, 4000000), 2000000, name), "a full list takes no more");
}

void item_search()
{
	using namespace ms::pets;

	require(search_key(" Red  Snail Shell ") == "redsnailshell", "lower case without spaces");
	require(search_key("   ").empty(), "spaces alone search nothing");
	require(searchable(4000000) && searchable(1302000) && searchable(2000000), "ordinary items are offered");

	for (int32_t id : { 9110000, 5550000, 5010000, 5020000, 5160000, 5430000, 1112000, 1112899, 5109488 })
		require(!searchable(id), "families the search leaves out");
}

int main()
{
	try
	{
		compacting_list();
		summoning();
		stances();
		spawn_player_pets();
		words();
		hearing();
		actions();
		food();
		speech();
		naming();
		equipment();
		info_page();
		exception_list();
		item_search();

		std::cout << "Pets passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
