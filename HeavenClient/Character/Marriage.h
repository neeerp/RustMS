#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	class InPacket;

	// A couple's record, as MARRIAGE_RESULT 11 and the character data's
	// marriage list give it (0x30 bytes, report b052).
	struct MarriageRecord
	{
		int32_t id = 0;
		int32_t groom_id = 0;
		int32_t bride_id = 0;
		// 1 engaged, 3 married.
		int16_t status = 0;
		// The ring boxes the two were engaged with.
		int32_t groom_item = 0;
		int32_t bride_item = 0;
		std::string groom_name;
		std::string bride_name;

		bool married() const;
		// The other one's name, chosen by the player's own gender (0x8ef073).
		const std::string& partner(bool female) const;

		static MarriageRecord read(InPacket& recv);
	};

	// Where the partner is, as NOTIFY_MARRIED_PARTNER_MAP_TRANSFER last said
	// (ctx+0x300c and +0x3010, report b057). Kept across the player's own
	// map changes; map 0 is nowhere and id 0 no one.
	struct PartnerSpot
	{
		int32_t mapid = 0;
		int32_t id = 0;

		// Whether a character on the map gets the minimap's partner dot.
		bool is(int32_t cid) const;
		// Whether a world map spot of `spot_maps` shows the partner's heart:
		// it is the partner's but not the player's own on `own_map`
		// (0x9edb98).
		bool marks(const std::vector<int32_t>& spot_maps, int32_t own_map) const;
	};

	namespace marriage
	{
		constexpr int16_t ENGAGED = 1;
		constexpr int16_t MARRIED = 3;

		// MARRIAGE_RESULT's mode with the couple's record.
		constexpr int8_t RESULT_ENGAGED = 11;
		// The modes that end the couple, engaged or married; the client
		// forgets the record.
		constexpr int8_t RESULT_BROKEN = 13;
		constexpr int8_t RESULT_DIVORCED = 14;
		// The mode whose text the server sends.
		constexpr int8_t RESULT_NOTICE = 36;

		// The engagement ring boxes (224xxxx) a proposal is made with.
		bool is_ring_box(int32_t item_id);
		// The empty boxes and rings a couple holds (4031357 to 4031364), which
		// the player drops to break off (0x4f33ee).
		bool is_token(int32_t item_id);
		// The wedding rings 1112803, 1112806, 1112807 and 1112809.
		bool is_wedding_ring(int32_t item_id);
		// The tooltip's line naming the partner: 0x1069 on a token, 0x1068 on
		// a wedding ring, whatever the record's status (0x8ef061, 0x8e90a8).
		// Empty without a record or for any other item.
		std::string tooltip_line(int32_t item_id, const std::optional<MarriageRecord>& record, bool female);
		// The question before a token goes: 0x10a3, or 0x10a4 for a couple
		// whose record says married (0x4f3475).
		const char* break_question(bool married);
		// MARRIAGE_RESULT's text for a mode (0x75d9a1), or nullptr.
		const char* result_text(int8_t mode);
	}
}
