#include "SetEffect.h"

#include "../Constants.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The sets by id. The image's children come in the order of their names as strings, but a set's id is
		// its number.
		const std::vector<set_effect::Set> &sets()
		{
			static const std::vector<set_effect::Set> loaded = []
			{
				std::vector<std::pair<int32_t, nl::node>> nodes;

				for (nl::node src : nl::nx::effect["SetEff.img"])
				{
					int32_t id = string_conversion::or_default<int32_t>(src.name(), -1);

					if (id >= 0)
						nodes.emplace_back(id, src);
				}

				std::sort(nodes.begin(), nodes.end(), [](auto &a, auto &b) { return a.first < b.first; });

				std::vector<set_effect::Set> list;

				for (auto &[id, src] : nodes)
				{
					set_effect::Set set;

					if (src["cash"].get_integer() != 0)
						set.kind = set_effect::Kind::CASH;
					else if (src["multipet"])
						set.kind = set_effect::Kind::MULTIPET;

					for (nl::node group : src["info"])
					{
						std::vector<int32_t> items;

						for (nl::node item : group)
							items.push_back(static_cast<int32_t>(item.get_integer()));

						if (group.name() == "multipet")
							set.pets = items;
						else
							set.groups.push_back({ string_conversion::or_default<int32_t>(group.name(), 0), items });
					}

					list.push_back(set);
				}

				return list;
			}();

			return loaded;
		}
	}

	void SetEffect::set(int32_t set_id)
	{
		id = set_id;
		effect.reset();
		background.reset();
		shown = {};
		opacity = 1.0f;

		if (id < 0)
			return;

		nl::node src = nl::nx::effect["SetEff.img"][std::to_string(id)];
		effect = load(src["effect"]);
		// It is seen at once, and fades at the next update if the character doesn't stand.
		background = load(src["backgrnd"]);
	}

	int32_t SetEffect::get() const
	{
		return id;
	}

	std::optional<SetEffect::Layer> SetEffect::load(nl::node src)
	{
		// Some sets have an empty effect, and only a background.
		if (!src || src.size() == 0)
			return std::nullopt;

		Layer layer{ src, src["pos"].get_integer() == 1, src["fixed"].get_integer() != 0,
					 src["z"].get_integer(3) >= 0, static_cast<int16_t>(src["repeatStartIndex"].get_integer()) };
		layer.animation.play(Animation::End::LOOP, layer.loop_from);

		return layer;
	}

	void SetEffect::update(const CharLook &look, const std::array<int32_t, 3> &pets, bool standing)
	{
		now += Constants::TIMESTEP;

		Worn now_worn;
		now_worn.parts[0] = look.get_hair_id();

		const CharEquips &equips = look.get_equips();

		for (int16_t part = 1; part < static_cast<int16_t>(now_worn.parts.size()); part++)
			now_worn.parts[part] = equips.get_equip(static_cast<EquipSlot::Id>(part));

		// The look keeps one ring.
		for (int16_t ring : { 13, 15, 16 })
			now_worn.parts[ring] = now_worn.parts[EquipSlot::Id::RING1];

		now_worn.cash_weapon = look.get_cash_weapon();
		now_worn.pets = pets;
		now_worn.morphed = look.get_morph() != 0 || look.get_ghost() != 0;

		if (!matched || !(now_worn == worn))
		{
			worn = now_worn;
			matched = true;

			int32_t now_id = worn.morphed ? -1 : set_effect::match(sets(), worn.parts, worn.cash_weapon, worn.pets);

			// The set already shown goes on.
			if (now_id != id)
				set(now_id);
		}

		if (effect)
			effect->animation.update();

		if (!background)
			return;

		// It comes back from its first frame.
		if (set_effect::step(shown, standing, now))
			background->animation.play(Animation::End::LOOP, background->loop_from);

		opacity = set_effect::fade(opacity, shown.shown, Constants::TIMESTEP);
		background->animation.update();
	}

	void SetEffect::draw(bool in_front, Point<int16_t> feet, Point<int16_t> face, bool facing_right,
						 float alpha) const
	{
		if (effect && effect->in_front == in_front)
			effect->animation.draw(DrawArgument(effect->at_face ? face : feet, !effect->fixed && facing_right),
								   alpha);

		if (background && background->in_front == in_front && opacity > 0.0f)
			background->animation.draw(
				DrawArgument(background->at_face ? face : feet, !background->fixed && facing_right, opacity), alpha);
	}
}
