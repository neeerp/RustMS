#pragma once

#include <cstdint>

namespace ms
{
	// The server's hold on the UI during scripted scenes: OPEN_UI (220),
	// LOCK_UI (221) and DISABLE_UI (222).
	namespace ui_control
	{
		// The windows OPEN_UI can open (FUN_00a05ab8). HC has no Monster
		// Carnival (0x12), Ranking (0x1a), GM board (0x1d, 0x1e) or Title
		// (0x1f) window, and 0x20's window is not known.
		enum class Window
		{
			NONE,
			ITEMS,
			EQUIPS,
			STATS,
			SKILLS,
			KEYCONFIG,
			QUESTLOG,
			MONSTERBOOK,
			CHARINFO,
			GUILDBBS,
			ENERGYBAR,
			PARTYSEARCH,
			MAKER,
			FAMILY,
			FAMILYTREE,
			// The Ranking's refusal for beginners up to level 9.
			RANKING_NOTICE
		};

		// The Aran tutorial maps' info/fieldType, where only the quest log
		// opens (0xa05ad4).
		constexpr int32_t ARAN_TUTORIAL_FIELD = 0x16;

		inline Window open_window(int32_t id, int32_t job, int32_t level, int32_t fieldtype)
		{
			if (fieldtype == ARAN_TUTORIAL_FIELD && id != 6)
				return Window::NONE;

			switch (id)
			{
				case 0:
					return Window::ITEMS;
				case 1:
					return Window::EQUIPS;
				case 2:
					return Window::STATS;
				// 0x21 is Evan's skill window, which 3 also opens for Evans;
				// HC has one skill window.
				case 3:
				case 0x21:
					return Window::SKILLS;
				case 5:
					return Window::KEYCONFIG;
				case 6:
					return Window::QUESTLOG;
				case 9:
					return Window::MONSTERBOOK;
				case 0xa:
					return Window::CHARINFO;
				case 0xb:
					return Window::GUILDBBS;
				case 0x14:
					return Window::ENERGYBAR;
				case 0x16:
					return Window::PARTYSEARCH;
				case 0x17:
					return Window::MAKER;
				case 0x1a:
				{
					bool beginner = job == 0 || job == 1000 || job == 2000 || job == 2001;

					return beginner && level <= 9 ? Window::RANKING_NOTICE : Window::NONE;
				}
				case 0x1b:
					return Window::FAMILY;
				case 0x1c:
					return Window::FAMILYTREE;
				default:
					return Window::NONE;
			}
		}

		// While DISABLE_UI is on, other players' spawns (0xa0) and their
		// packets 0xb9 to 0xcc are dropped (FUN_00972100, FUN_009724f9).
		inline bool dropped_when_disabled(uint16_t opcode)
		{
			return opcode == 0xa0 || (opcode >= 0xb9 && opcode <= 0xcc);
		}

		struct State
		{
			// LOCK_UI: no HUD, keys or movement (ctx+0x30b0).
			bool locked = false;
			// DISABLE_UI: no other players (ctx+0x30b4).
			bool disabled = false;
		};

		// Cleared on entering the game.
		inline State &state()
		{
			static State current;
			return current;
		}
	}
}
