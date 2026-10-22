#include "Marriage.h"

#include "../Net/InPacket.h"

#include <algorithm>
#include <cstdlib>

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

	bool PartnerSpot::is(int32_t cid) const
	{
		return id != 0 && id == cid;
	}

	bool PartnerSpot::marks(const std::vector<int32_t>& spot_maps, int32_t own_map) const
	{
		auto has = [&](int32_t map) { return std::find(spot_maps.begin(), spot_maps.end(), map) != spot_maps.end(); };

		return mapid != 0 && has(mapid) && !has(own_map);
	}

	bool RingEffect::operator==(const RingEffect& other) const
	{
		return item_id == other.item_id && mark == other.mark && pair == other.pair && offset == other.offset;
	}

	namespace marriage
	{
		std::optional<MarriageRing> read_ring_looks(InPacket& recv)
		{
			// A crush or friendship block: the two ring serials and the item.
			for (int i = 0; i < 2; i++)
				if (recv.read_bool())
					recv.skip(8 + 8 + 4);

			if (!recv.read_bool())
				return std::nullopt;

			MarriageRing ring;
			ring.self_id = recv.read_int();
			ring.partner_id = recv.read_int();
			ring.item_id = recv.read_int();

			return ring;
		}

		std::optional<MarriageRing> own_ring(const std::optional<MarriageRecord>& record, bool female, bool wearing)
		{
			if (!record || !wearing)
				return std::nullopt;

			MarriageRing ring;
			ring.self_id = female ? record->bride_id : record->groom_id;
			ring.partner_id = female ? record->groom_id : record->bride_id;
			ring.item_id = record->groom_item;

			return ring;
		}

		std::vector<std::pair<int32_t, RingEffect>> ring_effects(const std::vector<RingWearer>& wearers)
		{
			std::vector<std::pair<int32_t, RingEffect>> effects;
			std::vector<bool> paired(wearers.size(), false);

			for (size_t i = 0; i < wearers.size(); i++)
			{
				for (size_t j = i + 1; j < wearers.size() && !paired[i]; j++)
				{
					const RingWearer& a = wearers[i];
					const RingWearer& b = wearers[j];

					if (paired[j] || (a.ring.partner_id != b.ring.self_id && b.ring.partner_id != a.ring.self_id))
						continue;

					paired[i] = paired[j] = true;

					const RingWearer& low = a.id < b.id ? a : b;
					const RingWearer& high = a.id < b.id ? b : a;
					Point<int16_t> gap = high.position - low.position;
					int16_t dx = static_cast<int16_t>(std::abs(gap.x()));
					int16_t dy = static_cast<int16_t>(std::abs(gap.y()));

					RingEffect effect;
					effect.item_id = low.ring.item_id;
					effect.mark = dx < 700 && dy < 500;

					RingEffect low_effect = effect;
					low_effect.pair = dx < 250 && dy < 100;
					low_effect.offset = Point<int16_t>(gap.x() / 2, gap.y() / 2);

					effects.emplace_back(low.id, low_effect);
					effects.emplace_back(high.id, effect);
				}
			}

			return effects;
		}

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
