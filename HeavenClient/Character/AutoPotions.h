#pragma once

#include <cstdint>

namespace ms
{
	// The potions the first pet out gives from beside its HP and MP pouches
	// (v83 FUN_0095b9a4, FUN_0095bbe0).
	namespace auto_potion
	{
		constexpr int32_t HP_POUCH = 1812002;
		constexpr int32_t MP_POUCH = 1812003;
		// The pet skill bit that lets a pet speak up on its own (autoSpeaking).
		constexpr uint16_t SPEAKS_UP = 0x100;

		// Whether `stat`, `delta` short, is under `alert` percent of `max`.
		// An alert of 0 turns the potion off.
		inline bool below(int32_t stat, int32_t delta, int32_t max, int32_t alert)
		{
			if (alert <= 0 || max <= 0)
				return false;

			return (static_cast<int64_t>(stat) - delta) * 100 / max < alert;
		}

		// The pets say they have no potion for the first three misses in a
		// row (0x95bb53); the chat line says so every time.
		inline bool pets_complain(int32_t misses)
		{
			return misses <= 3;
		}

		enum class Change
		{
			// 0: no potion, which the server is told.
			CLEAR,
			// Without the pouch, nothing happens.
			NONE,
			// An item that restores nothing of the pouch's kind.
			REFUSED,
			// The potion already set.
			SAME,
			SET
		};

		// What putting `itemid` beside a pouch does (FUN_007ff0d6,
		// FUN_007ff5fe): `pouch` whether the first pet wears it, `restores`
		// whether the item's spec heals its kind.
		inline Change change(int32_t itemid, bool pouch, bool restores, int32_t current)
		{
			if (itemid == 0)
				return Change::CLEAR;

			if (!pouch)
				return Change::NONE;

			if (!restores)
				return Change::REFUSED;

			return itemid == current ? Change::SAME : Change::SET;
		}

		constexpr const char *ONLY_HP = "Only usable on Auto HP Potion Pouch.";
		constexpr const char *ONLY_MP = "Only usable on Auto MP Potion Pouch.";
		constexpr const char *SET_HP =
				"Your pet will consume the Potion once when your HP goes below HP alert setting.";
		constexpr const char *SET_MP =
				"Your pet will consume the Potion once when your MP goes below MP alert setting.";
		constexpr const char *LACKING_HP = "You are lacking the HP Potion that your pet is supposed to use.";
		constexpr const char *LACKING_MP = "You are lacking the MP Potion that your pet is supposed to use.";
		constexpr const char *LEVEL_TOO_LOW = "Your level is too low to use the selected item.";
	}
}
