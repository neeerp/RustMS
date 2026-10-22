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

	RingWearer RingWearer::of(int32_t id, Point<int16_t> position, const MarriageRing& ring)
	{
		return { id, position, ring.self_id, ring.partner_id, ring.item_id };
	}

	RingWearer RingWearer::of(int32_t id, Point<int16_t> position, const PairRing& ring)
	{
		return { id, position, ring.ring_id, ring.partner_ring_id, ring.item_id };
	}

	bool RingEffect::operator==(const RingEffect& other) const
	{
		return item_id == other.item_id && mark == other.mark && pair == other.pair && offset == other.offset;
	}

	namespace marriage
	{
		std::optional<RingKind> ring_kind(int32_t item_id)
		{
			if (is_wedding_ring(item_id))
				return RingKind::WEDDING;

			if (item_id / 100 == 11120 && item_id != 1112000)
				return RingKind::CRUSH;

			if (item_id / 100 == 11128 && item_id % 10 <= 2)
				return RingKind::FRIENDSHIP;

			return std::nullopt;
		}

		bool is_pair_ring(int32_t item_id)
		{
			std::optional<RingKind> kind = ring_kind(item_id);

			return kind == RingKind::CRUSH || kind == RingKind::FRIENDSHIP;
		}

		RingLooks read_ring_looks(InPacket& recv)
		{
			auto pair_ring = [&recv]() -> std::optional<PairRing>
			{
				if (!recv.read_bool())
					return std::nullopt;

				PairRing ring;
				ring.ring_id = recv.read_long();
				ring.partner_ring_id = recv.read_long();
				ring.item_id = recv.read_int();

				return ring;
			};

			RingLooks looks;
			looks.crush = pair_ring();
			looks.friendship = pair_ring();

			if (recv.read_bool())
			{
				MarriageRing ring;
				ring.self_id = recv.read_int();
				ring.partner_id = recv.read_int();
				ring.item_id = recv.read_int();
				looks.marriage = ring;
			}

			return looks;
		}

		CoupleRingRecord read_couple_ring(InPacket& recv, bool friendship)
		{
			CoupleRingRecord record;
			record.partner_id = recv.read_int();
			record.partner_name = recv.read_padded_string(13);
			record.ring_id = recv.read_long();
			record.partner_ring_id = recv.read_long();

			if (friendship)
				record.item_id = recv.read_int();

			return record;
		}

		std::optional<PairRing> own_pair_ring(RingKind kind, const std::vector<std::pair<int32_t, int64_t>>& worn,
											  const std::vector<CoupleRingRecord>& records)
		{
			auto ring = std::find_if(worn.begin(), worn.end(), [kind](auto& item) { return ring_kind(item.first) == kind; });

			if (ring == worn.end())
				return std::nullopt;

			for (const CoupleRingRecord& record : records)
				if (record.ring_id == ring->second)
					return PairRing{ record.ring_id, record.partner_ring_id, ring->first };

			return std::nullopt;
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

		std::vector<std::pair<int32_t, RingEffect>> ring_effects(const std::vector<RingWearer>& wearers, RingKind kind)
		{
			int16_t close = kind == RingKind::WEDDING ? 250 : 100;

			std::vector<std::pair<int32_t, RingEffect>> effects;
			std::vector<bool> paired(wearers.size(), false);

			for (size_t i = 0; i < wearers.size(); i++)
			{
				for (size_t j = i + 1; j < wearers.size() && !paired[i]; j++)
				{
					const RingWearer& a = wearers[i];
					const RingWearer& b = wearers[j];

					if (paired[j] || (a.partner != b.self && b.partner != a.self))
						continue;

					paired[i] = paired[j] = true;

					const RingWearer& low = a.id < b.id ? a : b;
					const RingWearer& high = a.id < b.id ? b : a;
					Point<int16_t> gap = high.position - low.position;
					int16_t dx = static_cast<int16_t>(std::abs(gap.x()));
					int16_t dy = static_cast<int16_t>(std::abs(gap.y()));

					RingEffect effect;
					effect.item_id = low.item_id;
					effect.mark = dx < 700 && dy < 500;

					RingEffect low_effect = effect;
					low_effect.pair = dx < close && dy < 100;
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

		std::string pair_tooltip_line(int32_t item_id, int64_t cash_id, const std::vector<CoupleRingRecord>& records)
		{
			std::optional<RingKind> kind = ring_kind(item_id);

			if (cash_id == 0 || (kind != RingKind::CRUSH && kind != RingKind::FRIENDSHIP))
				return "";

			for (const CoupleRingRecord& record : records)
				if (record.ring_id == cash_id)
					return kind == RingKind::CRUSH ? "A COUPLE-RING WITH : " + record.partner_name
												   : "This is your Friendship Ring with '" + record.partner_name + "'.";

			return "";
		}

		const char* equip_gate(int32_t item_id, int32_t replaced, const std::vector<int32_t>& worn)
		{
			std::optional<RingKind> kind = ring_kind(item_id);

			if (!kind || ring_kind(replaced) == kind
				|| std::none_of(worn.begin(), worn.end(), [&](int32_t id) { return ring_kind(id) == kind; }))
				return nullptr;

			switch (*kind)
			{
			case RingKind::CRUSH:
				return "Only one couple-ring can be equipped.";
			case RingKind::FRIENDSHIP:
				return "You may only equip\\none Friendship Ring at a time.";
			default:
				return "You may only equip ONE wedding ring.";
			}
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
