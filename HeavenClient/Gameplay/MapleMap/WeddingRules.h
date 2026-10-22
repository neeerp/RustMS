#pragma once

#include <cstdint>
#include <string>
#include <utility>

namespace ms
{
	// The priest's lines and the hearts at a wedding's altar: what
	// WEDDING_PROGRESS and WEDDING_CEREMONY_END show, and what the couple's
	// client says back (0x58153d and 0x581b49, report b075).
	namespace wedding
	{
		// The two altars; the client knows each one's priest itself.
		constexpr int32_t CHAPEL = 680000110;
		constexpr int32_t CATHEDRAL = 680000210;
		constexpr int32_t CHAPEL_PRIEST = 9201011;
		constexpr int32_t CATHEDRAL_PRIEST = 9201002;

		// The Cathedral's step on which guests may bless the couple.
		constexpr uint8_t BLESSING = 2;

		// The hearts float this far above the couple's midpoint.
		constexpr int16_t HEARTS_RISE = 20;

		// The priest of the altar `mapid`, or 0 on any other map.
		inline int32_t priest_of(int32_t mapid)
		{
			switch (mapid)
			{
				case CHAPEL:
					return CHAPEL_PRIEST;
				case CATHEDRAL:
					return CATHEDRAL_PRIEST;
				default:
					return 0;
			}
		}

		// The music step 0 starts, looped (pool 0x106a, 0x106b).
		inline std::string bgm_of(int32_t mapid)
		{
			return mapid == CATHEDRAL ? "BgmGL.img/cathedral" : "BgmGL.img/chapel";
		}

		// The priest's line for a step: String/Npc.img/<priest>/wedding<step>.
		inline std::string line_of(uint8_t step)
		{
			return "wedding" + std::to_string(step);
		}

		enum class Page
		{
			// The priest's line, with NEXT and End Chat.
			SAY,
			// "Do you want to bless this couple?" (pool 0x106c); Yes sends
			// WEDDING_BLESS.
			BLESS,
			// The couple see nothing while the guests bless them.
			NONE
		};

		// What a viewer sees for `step`; `couple` is whether the viewer is
		// the groom or the bride.
		inline Page page_for(int32_t mapid, uint8_t step, bool couple)
		{
			if (mapid == CATHEDRAL && step == BLESSING)
				return couple ? Page::NONE : Page::BLESS;

			return Page::SAY;
		}

		// Where the hearts stand: midway between the groom and the bride,
		// raised a little (0x581dc5).
		inline std::pair<int32_t, int32_t> hearts_at(int32_t groomx, int32_t groomy, int32_t bridex, int32_t bridey)
		{
			return { (groomx + bridex) / 2, (groomy + bridey) / 2 - HEARTS_RISE };
		}

		// The priest's pages still waiting to return. v83 runs each page as
		// a modal dialog from the packet's handler, so a page that a newer
		// packet ends returns only once the page that replaced it does; the
		// couple's client then says WEDDING_TALK, with the latest step, once
		// for each.
		class Pages
		{
		public:
			// A page goes up, over the one open if any.
			void show()
			{
				++waiting;
			}

			bool open() const
			{
				return waiting > 0;
			}

			// The open page is answered, or ended with nothing to replace
			// it: every page waiting returns. How many WEDDING_TALKs the
			// client sends; none from a guest, nor while the Cathedral's
			// guests bless the couple (0x581ab8).
			uint8_t settle(int32_t mapid, uint8_t latest, bool couple)
			{
				uint8_t returned = waiting;
				waiting = 0;

				if (!couple || (mapid == CATHEDRAL && latest == BLESSING))
					return 0;

				return returned;
			}

			// Another dialog took the page's place: nothing returns.
			void forget()
			{
				waiting = 0;
			}

		private:
			uint8_t waiting = 0;
		};
	}
}
