#pragma once

#include <cstdint>
#include <string>

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
		// The other one's name.
		const std::string& partner(int32_t self) const;

		static MarriageRecord read(InPacket& recv);
	};

	namespace marriage
	{
		constexpr int16_t ENGAGED = 1;
		constexpr int16_t MARRIED = 3;

		// MARRIAGE_RESULT's mode with the couple's record.
		constexpr int8_t RESULT_ENGAGED = 11;
		// The mode whose text the server sends.
		constexpr int8_t RESULT_NOTICE = 36;

		// The engagement ring boxes (224xxxx) a proposal is made with.
		bool is_ring_box(int32_t item_id);
		// MARRIAGE_RESULT's text for a mode (0x75d9a1), or nullptr.
		const char* result_text(int8_t mode);
	}
}
