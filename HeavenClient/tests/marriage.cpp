#include "../Character/Marriage.h"
#include "../Net/InPacket.h"
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

	// A name in its 13 bytes.
	Writer& padded(const std::string& name)
	{
		bytes.insert(bytes.end(), name.begin(), name.end());
		bytes.insert(bytes.end(), 13 - name.size(), 0);
		return *this;
	}

	InPacket packet() const
	{
		return InPacket(bytes.data(), bytes.size());
	}
};

void reads_the_couples_record()
{
	Writer writer;
	writer.put<int32_t>(7).put<int32_t>(100).put<int32_t>(200).put<int16_t>(1);
	writer.put<int32_t>(2240000).put<int32_t>(2240001).padded("Groom").padded("Bridetwelvech");
	require(writer.bytes.size() == 0x30, "the record is 0x30 bytes");

	InPacket recv = writer.packet();
	MarriageRecord record = MarriageRecord::read(recv);
	require(!recv.available(), "reads all of it");
	require(record.id == 7 && record.groom_id == 100 && record.bride_id == 200, "the ids");
	require(record.status == marriage::ENGAGED && !record.married(), "engaged");
	require(record.groom_item == 2240000 && record.bride_item == 2240001, "the boxes");
	require(record.groom_name == "Groom" && record.bride_name == "Bridetwelvech", "the names without padding");
	require(record.partner(false) == "Bridetwelvech" && record.partner(true) == "Groom", "each one's partner, by own gender");

	record.status = marriage::MARRIED;
	require(record.married(), "married");
}

void ring_boxes_and_result_texts()
{
	require(marriage::is_ring_box(2240000) && marriage::is_ring_box(2240003), "the boxes");
	require(!marriage::is_ring_box(2230000) && !marriage::is_ring_box(4031357), "not the boxes");

	require(std::string(marriage::result_text(marriage::RESULT_ENGAGED)) == "You are now engaged.", "engaged");
	require(std::string(marriage::result_text(marriage::RESULT_MARRIED)) == "You are now married!", "married");
	require(std::string(marriage::result_text(18)) == "You have entered the wrong character name.", "wrong name");
	require(std::string(marriage::result_text(30)) == "She has politely declined your engagement request.", "declined");

	for (int8_t mode = 18; mode <= 30; mode++)
		require(marriage::result_text(mode) != nullptr, "every refusal has its text");

	require(!marriage::result_text(17) && !marriage::result_text(33), "no text around them");
	require(!marriage::result_text(marriage::RESULT_INVITATION), "the invitation is a window");
	require(std::string(marriage::result_text(16)).rfind("Congratulations!", 0) == 0, "booked");
	require(std::string(marriage::result_text(31)).rfind("The reservation has been canceled", 0) == 0, "dropped");
	require(std::string(marriage::result_text(34)) == "This invitation is not valid.", "not valid");
	require(std::string(marriage::result_text(marriage::RESULT_BROKEN)) == "Your engagement has been broken.", "broken");
	require(std::string(marriage::result_text(marriage::RESULT_DIVORCED)) == "You are no longer married.", "divorced");
	require(marriage::result_text(32) != nullptr, "reserved");

	require(marriage::is_token(4031357) && marriage::is_token(4031364), "the tokens");
	require(!marriage::is_token(4031356) && !marriage::is_token(4031365), "not the tokens");
	require(std::string(marriage::break_question(false)).rfind("Dropping this item", 0) == 0, "engaged question");
	require(std::string(marriage::break_question(true)).rfind("You will not", 0) == 0, "married question");
	require(!marriage::result_text(marriage::RESULT_NOTICE), "the server's own text");
}

void invitations()
{
	require(marriage::is_invitation(4031377) && marriage::is_invitation(4031395), "the couple's invitations");
	require(!marriage::is_invitation(4031406) && !marriage::is_invitation(4031378), "not the couple's");
	require(marriage::is_received_invitation(4031406) && marriage::is_received_invitation(4031407), "a guest's");
	require(!marriage::is_received_invitation(4031395), "not a guest's");

	MarriageRecord record;
	record.status = marriage::ENGAGED;
	record.groom_name = "Groom";
	record.bride_name = "Bride";
	std::optional<MarriageRecord> couple = record;
	require(marriage::invitation_refusal(couple, "Guest") == nullptr, "a guest");
	require(std::string(marriage::invitation_refusal(couple, "Bride")).rfind("You may not send", 0) == 0, "the bride");
	require(std::string(marriage::invitation_refusal(couple, "Groom")).rfind("You may not send", 0) == 0, "the groom");
	// The client compares exactly; the server catches the rest.
	require(marriage::invitation_refusal(couple, "bride") == nullptr, "another case");
	require(std::string(marriage::invitation_refusal(std::nullopt, "Guest")).empty(), "no couple, no word");
	record.status = marriage::MARRIED;
	require(std::string(marriage::invitation_refusal(record, "Guest")).empty(), "married, no word");

	require(std::string(marriage::invitation_art(2)) == "Cathedral", "the Cathedral's");
	require(std::string(marriage::invitation_art(1)) == "Vegas" && std::string(marriage::invitation_art(0)) == "Vegas",
		"the Chapel's otherwise");
}

void tooltip_lines()
{
	MarriageRecord record;
	record.groom_name = "Groom";
	record.bride_name = "Bride";
	std::optional<MarriageRecord> couple = record;

	require(marriage::tooltip_line(4031357, couple, false) == "YOU ARE ENGAGED WITH : Bride", "a token, the groom");
	require(marriage::tooltip_line(4031364, couple, true) == "YOU ARE ENGAGED WITH : Groom", "a token, the bride");
	require(marriage::tooltip_line(1112803, couple, false) == "YOU ARE MARRIED WITH : Bride", "a wedding ring");
	require(marriage::tooltip_line(1112809, couple, true) == "YOU ARE MARRIED WITH : Groom", "the last ring");
	require(marriage::tooltip_line(4031357, std::nullopt, false).empty(), "no record, no line");
	require(marriage::tooltip_line(1112804, couple, false).empty(), "not a wedding ring");
	require(marriage::tooltip_line(2240000, couple, false).empty(), "a ring box has none");
}

void partner_spots()
{
	PartnerSpot none;
	require(!none.is(0), "id 0 is no one");
	require(!none.marks({ 0 }, 100000000), "map 0 is nowhere");

	PartnerSpot spot;
	spot.mapid = 102000000;
	spot.id = 4;
	require(spot.is(4) && !spot.is(5), "only the partner gets the dot");
	require(spot.marks({ 102000000, 102000100 }, 100000000), "the partner's spot");
	require(!spot.marks({ 100000000 }, 100000000), "another spot");
	require(!spot.marks({ 102000000, 102000100 }, 102000100), "the player's own spot wins");
}

void ring_looks()
{
	Writer none;
	none.put<int8_t>(0).put<int8_t>(0).put<int8_t>(0);
	InPacket recv = none.packet();
	RingLooks empty = marriage::read_ring_looks(recv);
	require(!empty.crush && !empty.friendship && !empty.marriage && !recv.available(), "three empty blocks");

	Writer worn;
	worn.put<int8_t>(1).put<int64_t>(11).put<int64_t>(12).put<int32_t>(1112001);
	worn.put<int8_t>(0);
	worn.put<int8_t>(1).put<int32_t>(5).put<int32_t>(4).put<int32_t>(1112806);
	recv = worn.packet();
	RingLooks looks = marriage::read_ring_looks(recv);
	require(!recv.available(), "all read");
	require(looks.crush && looks.crush->ring_id == 11 && looks.crush->partner_ring_id == 12
		&& looks.crush->item_id == 1112001, "the crush block");
	require(!looks.friendship, "no friendship block");
	std::optional<MarriageRing> ring = looks.marriage;
	require(ring && ring->self_id == 5 && ring->partner_id == 4 && ring->item_id == 1112806, "the marriage block");

	MarriageRecord record;
	record.groom_id = 4;
	record.bride_id = 5;
	record.groom_item = 1112803;
	require(!marriage::own_ring(record, false, false), "not without the ring worn");
	require(!marriage::own_ring(std::nullopt, false, true), "not without a couple");
	std::optional<MarriageRing> groom = marriage::own_ring(record, false, true);
	require(groom && groom->self_id == 4 && groom->partner_id == 5 && groom->item_id == 1112803, "the groom's");
	std::optional<MarriageRing> bride = marriage::own_ring(record, true, true);
	require(bride && bride->self_id == 5 && bride->partner_id == 4, "the bride's");
}

void couple_rings()
{
	require(marriage::ring_kind(1112001) == RingKind::CRUSH && marriage::ring_kind(1112099) == RingKind::CRUSH,
		"crush rings");
	require(!marriage::ring_kind(1112000) && !marriage::ring_kind(1112100), "a plain ring");
	require(marriage::ring_kind(1112800) == RingKind::FRIENDSHIP && marriage::ring_kind(1112812) == RingKind::FRIENDSHIP,
		"friendship rings");
	require(marriage::ring_kind(1112803) == RingKind::WEDDING && !marriage::ring_kind(1112804), "the 1128xx between");

	Writer lists;
	lists.put<int32_t>(9).padded("Crush").put<int64_t>(21).put<int64_t>(22);
	lists.put<int32_t>(8).padded("Friend").put<int64_t>(31).put<int64_t>(32).put<int32_t>(1112800);
	InPacket recv = lists.packet();
	std::vector<CoupleRingRecord> records = { marriage::read_couple_ring(recv, false),
		marriage::read_couple_ring(recv, true) };
	require(!recv.available(), "0x21 and 0x25 bytes");
	require(records[0].partner_id == 9 && records[0].partner_name == "Crush" && records[0].ring_id == 21
		&& records[0].partner_ring_id == 22, "a crush record");
	require(records[1].partner_name == "Friend" && records[1].item_id == 1112800, "a friendship record");

	std::optional<PairRing> crush = marriage::own_pair_ring(RingKind::CRUSH, { { 1112800, 31 }, { 1112001, 21 } }, records);
	require(crush && crush->ring_id == 21 && crush->partner_ring_id == 22 && crush->item_id == 1112001, "the worn crush ring");
	std::optional<PairRing> friendship = marriage::own_pair_ring(RingKind::FRIENDSHIP, { { 1112800, 31 } }, records);
	require(friendship && friendship->partner_ring_id == 32, "the worn friendship ring");
	require(!marriage::own_pair_ring(RingKind::CRUSH, { { 1112002, 99 }, { 1112001, 21 } }, records),
		"only the first crush ring counts");
	require(!marriage::own_pair_ring(RingKind::CRUSH, {}, records), "none worn");

	require(marriage::pair_tooltip_line(1112001, 21, records) == "A COUPLE-RING WITH : Crush", "a crush ring's line");
	require(marriage::pair_tooltip_line(1112800, 31, records) == "This is your Friendship Ring with 'Friend'.",
		"a friendship ring's line");
	require(marriage::pair_tooltip_line(1112001, 99, records).empty(), "not of a pair");
	require(marriage::pair_tooltip_line(1112001, 0, records).empty(), "no serial");

	require(!marriage::equip_gate(1112001, 0, { 1112800, 1112803 }), "one of each kind");
	const char* second = marriage::equip_gate(1112001, 0, { 0, 1112002 });
	require(second && second == std::string("Only one couple-ring can be equipped."),
		"a second crush ring");
	require(!marriage::equip_gate(1112001, 1112002, { 1112002 }), "in place of one");
	require(marriage::equip_gate(1112801, 1112001, { 1112001, 1112800 }) != nullptr, "a second friendship ring");
	require(marriage::equip_gate(1112806, 0, { 1112803 }) != nullptr, "a second wedding ring");
	require(!marriage::equip_gate(1112000, 0, { 1112000 }), "plain rings");
}

void ring_effects()
{
	auto wear = [](int32_t id, Point<int16_t> position, MarriageRing ring) { return RingWearer::of(id, position, ring); };
	MarriageRing groom{ 4, 5, 1112803 };
	MarriageRing bride{ 5, 4, 1112803 };
	MarriageRing other{ 7, 8, 1112806 };

	auto effects = marriage::ring_effects({ wear(5, Point<int16_t>(300, 0), bride), wear(7, Point<int16_t>(0, 0), other),
		wear(4, Point<int16_t>(100, 0), groom) });
	require(effects.size() == 2, "only the couple");
	auto [low, low_effect] = effects[0];
	auto [high, high_effect] = effects[1];
	require(low == 4 && high == 5, "the lower id first");
	require(low_effect.mark && high_effect.mark, "both marked on one screen");
	require(low_effect.pair && low_effect.offset == Point<int16_t>(100, 0), "close: the big effect midway");
	require(!high_effect.pair && low_effect.item_id == 1112803, "on the lower id only");

	auto couple = [&](Point<int16_t> gap) {
		return marriage::ring_effects({ wear(4, Point<int16_t>(0, 0), groom), wear(5, gap, bride) });
	};
	require(couple(Point<int16_t>(250, 0))[0].second.mark && !couple(Point<int16_t>(250, 0))[0].second.pair,
		"250 apart is not close");
	auto far = couple(Point<int16_t>(0, 500));
	require(!far[0].second.mark && !far[1].second.mark, "500 below is off the screen");
	require(couple(Point<int16_t>(249, 99))[0].second.pair, "249 across and 99 down is close");
	require(!couple(Point<int16_t>(0, 100))[0].second.pair, "100 down is not close");
	auto screen = couple(Point<int16_t>(-699, 499));
	require(screen[0].second.mark && screen[1].second.mark, "699 across and 499 down share a screen");
	require(!couple(Point<int16_t>(700, 0))[0].second.mark, "700 across is off the screen");

	// A crush pair goes by the ring serials, and is close within 100 px.
	PairRing mine{ 21, 22, 1112001 };
	PairRing theirs{ 22, 21, 1112001 };
	auto crush = [&](Point<int16_t> gap) {
		return marriage::ring_effects({ RingWearer::of(9, Point<int16_t>(0, 0), mine), RingWearer::of(3, gap, theirs) },
			RingKind::CRUSH);
	};
	auto near = crush(Point<int16_t>(99, 99));
	require(near.size() == 2 && near[0].first == 3 && near[0].second.pair, "99 across and down is close");
	require(near[0].second.offset == Point<int16_t>(-49, -49), "midway from the lower id");
	require(!crush(Point<int16_t>(100, 0))[0].second.pair && crush(Point<int16_t>(100, 0))[0].second.mark,
		"100 across is not");
	PairRing stranger{ 23, 24, 1112001 };
	require(marriage::ring_effects({ RingWearer::of(9, Point<int16_t>(0, 0), mine),
		RingWearer::of(3, Point<int16_t>(0, 0), stranger) }, RingKind::CRUSH).empty(), "another pair's ring");
}

int main()
{
	try
	{
		reads_the_couples_record();
		ring_boxes_and_result_texts();
		invitations();
		tooltip_lines();
		partner_spots();
		ring_looks();
		couple_rings();
		ring_effects();
	}
	catch (const std::exception& error)
	{
		std::cerr << "marriage: " << error.what() << "\n";
		return 1;
	}

	std::cout << "marriage: all passed\n";
	return 0;
}
