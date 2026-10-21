#pragma once

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>

namespace ms
{
	// The keys of the status bar's eight quick slots, left to right and top
	// to bottom (report 240). Each is a key map index (a scan code). v83
	// keeps them for the life of the process, with a saved copy that Key
	// Config reverts to on cancel (DAT_00bed658: +4 live, +0x24 saved).
	class QuickSlots
	{
	public:
		using Keys = std::array<int32_t, 8>;

		// Shift, Insert, Home, Page Up over Ctrl, Delete, End, Page Down
		// (table 0xbdafac).
		static constexpr Keys DEFAULTS = { 42, 82, 71, 73, 29, 83, 79, 81 };

		// Right Shift has no binding of its own; the game reads it as the
		// left one (FUN_008d639b, FUN_0072df55).
		static constexpr int32_t RIGHT_SHIFT = 54;
		static constexpr int32_t LEFT_SHIFT = 42;

		enum class Assign
		{
			OK,
			// "The key you just pressed is a key that has been disabled."
			DISABLED,
			// "This key is already being used."
			IN_USE
		};

		static int32_t fold(int32_t key)
		{
			return key == RIGHT_SHIFT ? LEFT_SHIFT : key;
		}

		// A key a quick slot may take: one with a label under
		// UIWindow.img/KeyConfig/quickslotConfig/key (FUN_0072dff3).
		static bool allowed(int32_t key)
		{
			return (key >= 2 && key <= 13) || (key >= 16 && key <= 27) || (key >= 29 && key <= 52) ||
				key == 56 || key == 57 || (key >= 59 && key <= 68) || key == 71 || key == 73 ||
				key == 79 || (key >= 81 && key <= 83) || key == 87 || key == 88;
		}

		// The editor's checks for pressing `key` on slot `slot` (FUN_0072df55):
		// a key without a label, then one any slot already has, the slot
		// itself included.
		static Assign assign(Keys &keys, size_t slot, int32_t key)
		{
			key = fold(key);

			if (!allowed(key))
				return Assign::DISABLED;

			if (std::find(keys.begin(), keys.end(), key) != keys.end())
				return Assign::IN_USE;

			keys[slot] = key;

			return Assign::OK;
		}

		// The key map index slot `slot` shows.
		int32_t key(size_t slot) const
		{
			return fold(live[slot]);
		}

		const Keys &keys() const
		{
			return live;
		}

		// QUICKSLOT_INIT (FUN_0072b82a) sets both copies.
		void init(const Keys &keys)
		{
			live = keys;
			saved = keys;
		}

		// The editor's OK (FUN_00836c9b).
		void set(const Keys &keys)
		{
			live = keys;
		}

		// Key Config's Default (FUN_0072b8e6).
		void reset()
		{
			live = DEFAULTS;
		}

		bool changed() const
		{
			return live != saved;
		}

		// Sending QUICK_SLOT makes the layout the saved one (FUN_0072b87e).
		void save()
		{
			saved = live;
		}

		// Closing Key Config without saving (FUN_00832834).
		void revert()
		{
			live = saved;
		}

	private:
		Keys live = DEFAULTS;
		Keys saved = DEFAULTS;
	};
}
