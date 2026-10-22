#include "Marriage.h"

#include "../Net/InPacket.h"

namespace ms
{
	bool MarriageRecord::married() const
	{
		return status == marriage::MARRIED;
	}

	const std::string& MarriageRecord::partner(int32_t self) const
	{
		return self == groom_id ? bride_name : groom_name;
	}

	MarriageRecord MarriageRecord::read(InPacket& recv)
	{
		MarriageRecord record;
		record.id = recv.read_int();
		record.groom_id = recv.read_int();
		record.bride_id = recv.read_int();
		record.status = recv.read_short();
		record.groom_item = recv.read_int();
		record.bride_item = recv.read_int();
		record.groom_name = recv.read_padded_string(13);
		record.bride_name = recv.read_padded_string(13);

		return record;
	}

	namespace marriage
	{
		bool is_ring_box(int32_t item_id)
		{
			return item_id / 10000 == 224;
		}

		// StringPool 0x108b and 0x1090 to 0x109a.
		const char* result_text(int8_t mode)
		{
			switch (mode)
			{
			case RESULT_ENGAGED: return "You are now engaged.";
			case 18: return "You have entered the wrong character name.";
			case 19: return "Your partner has to be in the same map.";
			case 20: return "Your ETC slot is full.\\nPlease remove some items.";
			case 21: return "Your partner's ETC slots are full.";
			case 22: return "You cannot be engaged to the same gender.";
			case 23: return "You are already engaged.";
			case 24: return "She is already engaged.";
			case 25: return "You are already married.";
			case 26: return "This person is already married.";
			case 27: return "You're already in middle or proposing a person.";
			case 28: return "She is currently being asked by another suitor.";
			case 29: return "Unfortunately, the man who proposed to you has withdrawn his request for an engagement.";
			case 30: return "She has politely declined your engagement request.";
			default: return nullptr;
			}
		}
	}
}
