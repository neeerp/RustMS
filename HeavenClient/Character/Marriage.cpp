#include "Marriage.h"

#include "../Net/InPacket.h"

namespace ms
{
	bool MarriageRecord::married() const
	{
		return status == marriage::MARRIED;
	}

	const std::string& MarriageRecord::partner(bool female) const
	{
		return female ? groom_name : bride_name;
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

		bool is_token(int32_t item_id)
		{
			return item_id >= 4031357 && item_id <= 4031364;
		}

		bool is_wedding_ring(int32_t item_id)
		{
			return item_id == 1112803 || item_id == 1112806 || item_id == 1112807 || item_id == 1112809;
		}

		std::string tooltip_line(int32_t item_id, const std::optional<MarriageRecord>& record, bool female)
		{
			if (!record)
				return "";

			if (is_token(item_id))
				return "YOU ARE ENGAGED WITH : " + record->partner(female);

			if (is_wedding_ring(item_id))
				return "YOU ARE MARRIED WITH : " + record->partner(female);

			return "";
		}

		const char* break_question(bool married)
		{
			return married ? "You will not be able to receive the wedding ring.\\nIs that okay?"
						   : "Dropping this item will break your engagement.\\nDo you still wish to drop this item?";
		}

		// StringPool 0x108b, 0x108e, 0x108f, 0x1090 to 0x109b.
		const char* result_text(int8_t mode)
		{
			switch (mode)
			{
			case RESULT_ENGAGED: return "You are now engaged.";
			case RESULT_BROKEN: return "Your engagement has been broken.";
			case RESULT_DIVORCED: return "You are no longer married.";
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
			case 32: return "You can't break the engagement after making reservations.";
			default: return nullptr;
			}
		}
	}
}
