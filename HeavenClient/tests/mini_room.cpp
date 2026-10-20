#include "../Net/Handlers/MiniRoom.h"
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

	Writer& head(int8_t type, int32_t itemid)
	{
		return put(type).put(itemid).put<int8_t>(0).put<int64_t>(0);
	}

	Writer& stack(int32_t itemid, int16_t count)
	{
		return head(2, itemid).put(count).string("").put<int16_t>(0);
	}

	Writer& equip(int32_t itemid)
	{
		head(1, itemid).put<int8_t>(7).put<int8_t>(0);

		for (int i = 0; i < 15; i++)
			put<int16_t>(static_cast<int16_t>(i));

		string("owner").put<int16_t>(0);
		put<int8_t>(0).put<int8_t>(1).put<int32_t>(0).put<int32_t>(0).put<int64_t>(0);
		return put<int64_t>(0).put<int32_t>(-1);
	}
};

// Stands in for the AvatarLook parser: a look here is just its hair.
ms::LookEntry read_hair(ms::InPacket& recv)
{
	ms::LookEntry look = {};
	look.hairid = recv.read_int();

	return look;
}

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::mini_room;

		// The inviter's room: two seats, ours is 0, and only us in it.
		Writer own;
		own.put<int8_t>(3).put<uint8_t>(2).put<uint8_t>(0);
		own.put<uint8_t>(0).put<int32_t>(30000).string("Alice");
		own.put<uint8_t>(0xFF);
		InPacket own_recv(own.bytes.data(), own.bytes.size());
		Enter alone = read_enter(own_recv, read_hair);

		require(!own_recv.available(), "an enter is read up to its end");
		require(alone.type == TRADING_ROOM && alone.max_users == 2 && alone.own_slot == 0, "the room and our seat are read");
		require(alone.members.size() == 1 && alone.members[0].name == "Alice", "a lone member is read");
		require(alone.members[0].look.hairid == 30000, "the look goes through the look reader");

		// The invitee comes in second.
		Writer both;
		both.put<int8_t>(3).put<uint8_t>(2).put<uint8_t>(1);
		both.put<uint8_t>(0).put<int32_t>(30000).string("Alice");
		both.put<uint8_t>(1).put<int32_t>(30010).string("Bob");
		both.put<uint8_t>(0xFF);
		InPacket both_recv(both.bytes.data(), both.bytes.size());
		Enter visit = read_enter(both_recv, read_hair);

		require(!both_recv.available() && visit.own_slot == 1, "the invitee's enter is read");
		require(visit.members.size() == 2 && visit.members[1].slot == 1, "both members are read with their slots");
		require(visit.members[1].name == "Bob" && visit.members[1].look.hairid == 30010, "the second member is read");

		Writer closed;
		closed.put<int8_t>(0).put<int8_t>(1);
		InPacket closed_recv(closed.bytes.data(), closed.bytes.size());
		Enter refused = read_enter(closed_recv, read_hair);

		require(!closed_recv.available() && refused.type == 0 && refused.error == 1, "an enter error is read");
		require(enter_error_text(refused.error) == "The room is already closed.", "a closed room says so");
		require(enter_error_text(9).find("same map") != std::string::npos, "a different map says so");
		require(enter_error_text(0x7F).empty(), "unknown errors say nothing");

		// Items and mesos name their side and slot.
		Writer mine;
		mine.put<uint8_t>(0).put<uint8_t>(1).equip(1302000);
		InPacket mine_recv(mine.bytes.data(), mine.bytes.size());
		auto sword = read_put_item(mine_recv);

		require(sword && !mine_recv.available(), "an equip is read past its stats");
		require(sword->side == MINE && sword->slot == 1 && sword->item.itemid == 1302000, "my item and its slot are read");
		require(sword->item.count == 1, "an equip is one");

		Writer theirs;
		theirs.put<uint8_t>(1).put<uint8_t>(9).stack(2000000, 25);
		InPacket theirs_recv(theirs.bytes.data(), theirs.bytes.size());
		auto potions = read_put_item(theirs_recv);

		require(potions && potions->side == PARTNER && potions->slot == 9, "the partner's item is read");
		require(potions->item.itemid == 2000000 && potions->item.count == 25, "a stack is read with its count");

		Writer outside;
		outside.put<uint8_t>(0).put<uint8_t>(10).stack(2000001, 1);
		InPacket outside_recv(outside.bytes.data(), outside.bytes.size());

		require(!read_put_item(outside_recv) && !outside_recv.available(), "a slot past nine is read and refused");

		Writer none;
		none.put<uint8_t>(0).put<uint8_t>(0).stack(2000001, 1);
		InPacket none_recv(none.bytes.data(), none.bytes.size());

		require(!read_put_item(none_recv), "slot zero is refused");

		Writer stranger;
		stranger.put<uint8_t>(2).put<uint8_t>(1).stack(2000001, 1);
		InPacket stranger_recv(stranger.bytes.data(), stranger.bytes.size());

		require(!read_put_item(stranger_recv), "a third side is refused");

		Writer meso;
		meso.put<uint8_t>(1).put<int32_t>(1500000);
		InPacket meso_recv(meso.bytes.data(), meso.bytes.size());
		auto offered = read_put_meso(meso_recv);

		require(offered && !meso_recv.available(), "mesos are read");
		require(offered->side == PARTNER && offered->meso == 1500000, "mesos name their side");

		Writer stray_meso;
		stray_meso.put<uint8_t>(5).put<int32_t>(1);
		InPacket stray_recv(stray_meso.bytes.data(), stray_meso.bytes.size());

		require(!read_put_meso(stray_recv) && !stray_recv.available(), "mesos of a third side are refused");

		std::array<Offer, 2> offers;
		offers[MINE].items[0] = sword->item;
		offers[PARTNER].items[8] = potions->item;

		// The Trade button lists the items slot by slot, mine first.
		offers[PARTNER].items[0] = storage_contents::Stored{ 4000000, 3 };
		offers[MINE].items[4] = storage_contents::Stored{ 2000002, 1 };

		std::vector<int32_t> listed = item_ids(offers);
		std::vector<int32_t> expected = { 1302000, 4000000, 2000002, 2000000 };

		require(listed == expected, "both sides are listed slot by slot");
		require(item_ids(offers[PARTNER]) == std::vector<int32_t>({ 4000000, 2000000 }), "one side is listed by slot");

		require(asks_count(2000000, 2) && asks_count(4000000, 100), "stacks ask how many");
		require(!asks_count(2000000, 1), "a single item goes whole");
		require(!asks_count(1302000, 2) && !asks_count(2070000, 500) && !asks_count(2330000, 2), "equips, stars and bullets go whole");

		// The grids: mine on the right, the partner's on the left.
		require(slot_at(172, 131) == 1 && slot_at(203, 162) == 1, "my first slot spans 32 pixels");
		require(slot_at(204, 131) == 0 && slot_at(171, 131) == 0, "the gaps between slots are no slot");
		require(slot_at(172 + 78, 131 + 74) == 9, "my last slot");
		require(slot_at(10, 131) == -1 && slot_at(10 + 78, 131 + 74) == -9, "the partner's slots");
		require(slot_at(172, 130) == 0 && slot_at(172, 131 + 74 + 32) == 0, "above and below the grid is no slot");
		require(slot_at(172 + 39, 131 + 37) == 5, "the slots go row by row");

		require(meso_text(0) == "0" && meso_text(999) == "999", "small amounts have no commas");
		require(meso_text(1000) == "1,000" && meso_text(1234567) == "1,234,567", "thousands are separated");
		require(meso_text(2147483647) == "2,147,483,647", "the largest amount");
		require(meso_text(-1000) == "-1,000", "the sign is not separated");

		require(invite_result_named(2) && invite_result_named(4) && !invite_result_named(1), "only some results carry a name");
		require(invite_result_text(1, "") == "Unable to find the character.", "a missing invitee");
		require(invite_result_text(3, "Bob") == "'Bob' have denied invitation.", "a declined invite names the invitee");
		require(invite_result_text(0, "Bob").empty(), "other results say nothing");

		// The fee message shows only when the partner's mesos came in short.
		std::array<Offer, 2> trade;
		trade[PARTNER].meso = 1000000;

		require(received_after_fees(500, 500 + 982000, trade) == 982000, "mesos after fees");
		require(received_after_fees(500, 500 + 1000000, trade) == 0, "no fee, no fee message");
		require(received_after_fees(500, 500, trade) == 0, "no mesos, no fee message");

		// Trade warns of the partner's Karma items, first listing those with a
		// time limit.
		constexpr int64_t MINUTE = 600000000;
		constexpr int64_t NOW = 1000 * 1440 * MINUTE;
		auto named = [](int32_t itemid) { return std::to_string(itemid); };
		Offer partner;
		partner.items[0] = storage_contents::Stored{ 4000000, 3 };
		partner.items[0]->expiration = NOW + MINUTE;
		TradeQuestions plain = trade_questions(partner, NOW, named);

		require(plain.time_limit.empty() && plain.confirm == "Are you sure you want to trade?",
			"items without Karma are only asked about");

		partner.items[3] = storage_contents::Stored{ 1302000, 1, 0x10 };
		TradeQuestions karma = trade_questions(partner, NOW, named);

		require(karma.time_limit.empty() && karma.confirm.rfind("Cash Items become untradable\\n", 0) == 0 &&
			karma.confirm.find("Do you really want to trade?") != std::string::npos,
			"a Karma item warns it can't be traded again");

		partner.items[3]->expiration = NOW + (2 * 1440 + 3 * 60 + 4) * MINUTE + MINUTE - 1;
		partner.items[5] = storage_contents::Stored{ 2000000, 1, 0x02 };
		partner.items[5]->expiration = NOW;
		partner.items[6] = storage_contents::Stored{ 1302000, 1, 0x02 };
		partner.items[6]->expiration = NOW;

		require(trade_questions(partner, NOW, named).time_limit ==
			"1302000(2days, 3hours, and 4minutes left)\\n2000000(Time Limit Expired)\\n"
			"\\nHave you checked the\\ntime limit on the item?",
			"timed Karma items are listed slot by slot with the whole days, hours and minutes left");

		trade[MINE].meso = 200000;

		require(received_after_fees(0, 790000, trade) == 790000, "my own offer counts against the partner's");
		require(received_after_fees(0, 800000, trade) == 0, "the difference in full is no fee");

		require(leave_text(7, 982000) == "Trade successful.\\nReceived 982000 mesos after fees.\\nPlease check the results.", "the fee message carries the mesos");
		require(leave_text(7, 0) == "Trade successful.\\nPlease check the results.", "a trade without fees");
		require(leave_text(2, 0) == "Trade cancelled.\\nby the other character.", "a cancelled trade");
		require(leave_text(8, 0) == "Trade unsuccessful." && leave_text(0, 0).empty(), "a failed trade, and leaving quietly");
		require(leave_text(0xa, 0) == "Trade was canceled because there was an item\\nthat disappeared because it expired."
			&& leave_text(0xb, 0) == "A cash item trade was canceled\\ndue to a server check.", "the expired and cash item reasons");

		auto stars = [](const std::string& text) { return std::string(text.size(), '*'); };

		require(filter_chat("Bob : darn it", stars) == "Bob : *******", "only the text after the name is filtered");
		require(filter_chat("Bob : a : b", stars) == "Bob : *****", "the first separator ends the name");
		require(filter_chat("darn", stars) == "****", "a line without a name is filtered whole");

		std::cout << "Mini room passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
