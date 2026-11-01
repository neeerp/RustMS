#pragma once

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <string>

// The rules behind v83's effect items, 501xxxx cash and 429xxxx etc items a character shows until they stop it
// (USE_ITEMEFFECT 0x34, SHOW_ITEM_EFFECT 194; FUN_00a24530 and FUN_0093b6a7).
namespace ms::item_effect
{
	// Only these two kinds have an effect.
	inline bool is_effect(int32_t item_id)
	{
		return item_id / 10000 == 501 || item_id / 10000 == 429;
	}

	// The effect's node: Item/<category>/<image>/%08d/effect.
	inline const char *category(int32_t item_id)
	{
		return item_id / 10000 == 429 ? "Etc" : "Cash";
	}

	inline std::string image(int32_t item_id)
	{
		return "0" + std::to_string(item_id / 10000) + ".img";
	}

	// Using the item shown stops it; using another shows that one instead.
	inline int32_t toggled(int32_t shown, int32_t item_id)
	{
		return shown == item_id ? 0 : item_id;
	}

	// An effect is shown at most once every 2 s from when the last one was. Stopping one is never held back.
	constexpr int64_t THROTTLE = 2000;

	inline bool may_send(int32_t next, int64_t now, int64_t last_shown)
	{
		return next == 0 || last_shown < 0 || now - last_shown >= THROTTLE;
	}

	// An action item has no actions on a tamed monster (190xxxx) or a vehicle (193xxxx) unless its `tamingMob`, 1
	// by default, says it keeps them there (0x93bb72); it plays `default` then.
	constexpr bool acts(bool keyed, bool taming_mob, int32_t vehicle)
	{
		int32_t kind = vehicle / 10000;

		return keyed && (taming_mob || (kind != 190 && kind != 193));
	}

	// An item with `action` set plays the node named after the action under way, and `default` in any action it
	// has no node for (FUN_0093beb9). The others always play `default`.
	inline std::string key(bool keyed, bool has_action, const std::string &action)
	{
		return keyed && has_action ? action : "default";
	}

	// A follower doesn't move while its target is this close (Shape2D's Vector2D looseness, FUN_51408d23).
	constexpr double DEAD_ZONE = 28.0;

	// One axis of a follower that trails its target with `loose` (0..4 children's `loose`): beyond the dead zone
	// it is left behind by L * d / (dt * d + L), L = loose * 1000, so it closes in over time. It is drawn at the
	// truncated value and keeps the exact one.
	struct Trail
	{
		double at = 0.0;
		int64_t since = 0;
		bool started = false;
	};

	inline int32_t trail(Trail &axis, int32_t target, int64_t now, int32_t loose)
	{
		if (!axis.started)
		{
			axis = { static_cast<double>(target), now, true };

			return target;
		}

		double distance = std::abs(target - axis.at);
		double lag = distance;

		if (distance > DEAD_ZONE)
		{
			double l = loose * 1000.0;
			double divisor = static_cast<double>(now - axis.since) * distance + l;
			lag = divisor > 0.0 ? std::max(0.0, l * distance / divisor) : 0.0;
		}

		if (target > axis.at)
			axis.at = target - lag;
		else if (target < axis.at)
			axis.at = target + lag;

		axis.since = now;

		return static_cast<int32_t>(axis.at);
	}

	// A follower faces the character: toward the right when the face is at or right of it.
	inline bool follower_faces_right(int32_t face_x, int32_t follower_x)
	{
		return face_x >= follower_x;
	}

	// Shadow Style (`spectrum`) leaves a ghost of the avatar every `interval` ms, which fades from `alpha` (of 255)
	// to nothing over `delay` ms (FUN_004433d1).
	inline float ghost_opacity(int32_t alpha, int64_t age, int32_t delay)
	{
		if (delay <= 0 || age >= delay)
			return 0.0f;

		float start = static_cast<float>(std::clamp(alpha, 0, 255)) / 255.0f;

		return start * (1.0f - static_cast<float>(age) / static_cast<float>(delay));
	}
}
