#include "ItemEffect.h"

#include "../Constants.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	void ItemEffect::set(int32_t id)
	{
		item_id = id;
		effect = {};
		keyed = false;
		action.clear();
		layer.reset();
		followers.clear();
		// The ghosts already left fade out on their own.
		spectrum = false;

		if (!item_effect::is_effect(id))
			return;

		effect = nl::nx::item[item_effect::category(id)][item_effect::image(id)][string_format::extend_id(id, 8)]["effect"];

		if (!effect)
			return;

		keyed = effect["action"].get_integer() != 0;
		spectrum = effect["spectrum"].get_integer() != 0;
		interval = static_cast<int32_t>(effect["interval"].get_integer(100));
		delay = static_cast<int32_t>(effect["delay"].get_integer(1000));
		ghost_alpha = static_cast<uint8_t>(effect["alpha"].get_integer(0x80));
		next_ghost = now;

		// Only `follow` on the effect itself counts; under `default` it is a plain effect.
		if (effect["follow"].get_integer() != 0)
		{
			for (int32_t i = 0; i < 5; i++)
			{
				nl::node child = effect[std::to_string(i)];

				if (!child)
					break;

				followers.push_back({ child, child["z"].get_integer(3) >= 0,
									  static_cast<int32_t>(child["loose"].get_integer()), {}, {}, {} });
			}
		} else if (!keyed)
		{
			load("default");
		}
	}

	int32_t ItemEffect::get() const
	{
		return item_id;
	}

	void ItemEffect::restart()
	{
		ghosts.clear();
		set(item_id);
	}

	void ItemEffect::load(const std::string &key)
	{
		// A missing node shows nothing; so does an empty one, as most action items' `default` is.
		nl::node src = effect[key];

		if (src)
			layer = Layer{ src, src["pos"].get_integer() == 1, src["fixed"].get_integer() != 0,
						   src["z"].get_integer(3) >= 0 };
		else
			layer.reset();
	}

	void ItemEffect::update(const std::string &now_action, Point<int16_t> feet, Point<int16_t> face,
							const CharLook &look)
	{
		now += Constants::TIMESTEP;

		// An action item starts its node over at every change of action, even from one that plays `default` to
		// another.
		if (keyed && now_action != action)
		{
			action = now_action;
			load(item_effect::key(true, static_cast<bool>(effect[action]), action));
		}

		if (layer)
			layer->animation.update();

		for (auto &follower : followers)
		{
			follower.animation.update();
			follower.at = Point<int16_t>(
				static_cast<int16_t>(item_effect::trail(follower.x, face.x(), now, follower.loose)),
				static_cast<int16_t>(item_effect::trail(follower.y, face.y(), now, follower.loose)));
		}

		if (spectrum && interval > 0 && now >= next_ghost)
		{
			next_ghost = now + interval;
			ghosts.push_back({ look, feet, now, ghost_alpha, delay });
		}

		while (!ghosts.empty() && now - ghosts.front().born >= ghosts.front().delay)
			ghosts.pop_front();
	}

	void ItemEffect::draw_ghosts(Point<int16_t> view) const
	{
		for (auto &ghost : ghosts)
		{
			float opacity = item_effect::ghost_opacity(ghost.alpha, now - ghost.born, ghost.delay);
			ghost.look.draw(DrawArgument(ghost.at + view, Color(1.0f, 1.0f, 1.0f, opacity)), 1.0f);
		}
	}

	void ItemEffect::draw(bool in_front, Point<int16_t> feet, Point<int16_t> face, bool facing_right,
						  Point<int16_t> view, float alpha) const
	{
		if (layer && layer->in_front == in_front)
			layer->animation.draw(DrawArgument(layer->at_face ? face : feet, !layer->fixed && facing_right), alpha);

		for (auto &follower : followers)
		{
			if (follower.in_front != in_front)
				continue;

			Point<int16_t> at = follower.at + view;
			follower.animation.draw(DrawArgument(at, item_effect::follower_faces_right(face.x(), at.x())), alpha);
		}
	}
}
