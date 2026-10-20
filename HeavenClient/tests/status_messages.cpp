#include "../Net/Handlers/StatusMessages.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
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

	// SHOW_STATUS_INFO mode 3 after the mode, as the server sends it.
	Writer& exp(bool white, int32_t gain, bool inchat, int32_t party)
	{
		put<int8_t>(white).put(gain).put<int8_t>(inchat).put<int32_t>(0).put<int16_t>(0).put<int32_t>(0);

		if (inchat)
			put<int8_t>(0);

		return put<int8_t>(0).put(party).put<int32_t>(0).put<int32_t>(0).put<int32_t>(0);
	}
};

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::status_messages;

		require(tab_name(4000019) == "Etc" && tab_name(2000000) == "Use" && tab_name(1302000) == "Eqp", "tabs by item id");
		require(item_line("Etc", "Snail Shell", 1) == "You have gained an item in the Etc tab (Snail Shell)", "one item");
		require(item_line("Etc", "Snail Shell", 3) == "You have gained items in the Etc tab (Snail Shell 3)", "the count follows the name");
		require(meso_line(150) == "You have gained mesos (+150)", "gaining mesos");
		require(meso_line(-150) == "You have lost mesos. (-150)", "losing mesos");
		require(fame_line(1) == "You have gained fame. (+1)", "gaining fame");
		require(fame_line(-1) == "You have lost fame (-1)", "losing fame");
		require(guild_points_line(5) == "You have earned Guild Points (+5)", "earning guild points");
		require(guild_points_line(0) == "You have lost Guild Points (0)", "losing none");

		Writer kill;
		kill.exp(true, 25, false, 3);
		InPacket recv(kill.bytes.data(), kill.bytes.size());
		ExpGain exp = read_exp(recv);
		require(!recv.available(), "the whole EXP message is read");
		require(exp.white && exp.gain == 25 && !exp.inchat && exp.party == 3, "a party kill's EXP");

		auto lines = exp_lines(exp);
		require(lines.size() == 2, "the gain, then the party bonus");
		require(lines[0].text == "You have gained experience (+25)" && !lines[0].yellow, "the gain is white");
		require(lines[1].text == "Bonus EXP for PARTY (+3)" && lines[1].yellow, "the party bonus is yellow");

		Writer solo;
		solo.exp(false, 7, false, 0);
		InPacket solo_recv(solo.bytes.data(), solo.bytes.size());
		auto solo_lines = exp_lines(read_exp(solo_recv));
		require(solo_lines.size() == 1 && solo_lines[0].yellow, "a solo kill has one line, yellow if not white");

		Writer quest;
		quest.exp(true, 10, true, 0);
		InPacket quest_recv(quest.bytes.data(), quest.bytes.size());
		ExpGain quest_exp = read_exp(quest_recv);
		require(!quest_recv.available(), "quest EXP carries one more byte");
		require(quest_exp.inchat && quest_exp.gain == 10, "quest EXP goes to the chat log");
		require(chat_exp_lines(quest_exp).size() == 1, "quest EXP without a Spirit Week bonus is one line");

		Writer bonus;
		bonus.put<int8_t>(1).put<int32_t>(100).put<int8_t>(0).put<int32_t>(4).put<int8_t>(10).put<int8_t>(0).put<int32_t>(0)
			.put<int8_t>(2).put<int8_t>(0).put<int32_t>(0).put<int32_t>(5).put<int32_t>(0).put<int32_t>(6);
		InPacket bonus_recv(bonus.bytes.data(), bonus.bytes.size());
		ExpGain bonus_exp = read_exp(bonus_recv);
		require(!bonus_recv.available(), "a hunting bonus carries its hours");
		auto bonus_lines = exp_lines(bonus_exp);
		require(bonus_lines.size() == 5, "the gain and four bonuses");
		require(bonus_lines[1].text == "Equip Item Bonus EXP (+5)", "the equipment bonus comes first");
		require(bonus_lines[2].text == "Bonus EXP for hunting over 2 hrs. (+10)", "the hunting bonus");
		require(bonus_lines[3].text == "Bonus Event EXP (+4)", "the event bonus");
		require(bonus_lines[4].text == "Rainbow Week Bonus EXP (+6)", "the rainbow week bonus comes last");

		int32_t last = 0;
		require(mob_bonus_line(exp, last).empty(), "no bonus percent, no line");
		require(mob_bonus_line(bonus_exp, last) == "A bonus EXP 10% is awarded for every 3rd monster defeated." &&
				last == 10, "a new bonus percent is told");
		require(mob_bonus_line(bonus_exp, last).empty() && mob_bonus_line(exp, last).empty() && last == 10,
				"once, until the percent changes");

		std::cout << "Status messages passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
