#pragma once

#include "../../Graphics/Text.h"

#include "../../Template/Point.h"
#include "../../Template/Range.h"

#include <cstdint>
#include <string>
#include <utility>

namespace ms
{
	// What UIUserList's guild and alliance tabs share: the layout, which
	// the alliance tab copies from the guild tab's, and a few text helpers.
	namespace UserListLayout
	{
		// A list's row and its column headings.
		inline constexpr int16_t COLUMNS = 18;
		inline constexpr int16_t ROW = 18;

		// The guild tab, as the client lays it out (FUN_0090f17c). The
		// client places text by its top; HeavenClient's 5 pixels lower.
		inline constexpr int16_t CLIENT_TEXT = -5;
		// The header: the guild's emblem and name, centred, then the notice.
		inline constexpr Point<int16_t> GUILD_HEADER = Point<int16_t>(5, 55);
		inline constexpr int16_t GUILD_HEADER_WIDTH = 306;
		inline constexpr int16_t EMBLEM_SPACE = 26;
		inline constexpr int16_t GUILD_NAME_Y = 3;
		inline constexpr Point<int16_t> NOTICE_BOX = Point<int16_t>(5, 80);
		inline constexpr Point<int16_t> NOTICE_LABEL = Point<int16_t>(13, 85);
		// A notice scrolls left through its strip a pixel each 30 ms update,
		// and starts over once it has left (FUN_00910dde).
		inline constexpr Point<int16_t> TICKER = Point<int16_t>(10, 85);
		inline constexpr int16_t TICKER_WIDTH = 270;
		inline constexpr int16_t TICKER_START = 250;
		// The alliance's notice starts further right.
		inline constexpr int16_t UNION_TICKER_START = 270;
		inline constexpr uint16_t TICKER_STEP = 30;
		// The member list scrolls through its view 33 pixels a step.
		inline constexpr Point<int16_t> GUILD_LIST = Point<int16_t>(6, 113);
		inline constexpr int16_t GUILD_VIEW = 192;
		inline constexpr int16_t GUILD_STEP = 33;
		inline constexpr Range<int16_t> GUILD_SLIDER = Range<int16_t>(113, 292);
		inline constexpr int16_t GUILD_SLIDER_X = 290;
		// Each section: a header, then, unless folded, column headings, a
		// page of rows (or "none") and an end line.
		inline constexpr int16_t GUILD_SECTION_HEADER = 27;
		inline constexpr int16_t GUILD_COLUMNS_GAP = 4;
		inline constexpr int16_t GUILD_END = 1;
		inline constexpr int16_t GUILD_SECTION_GAP = 10;
		inline constexpr int16_t MEMBERS_PER_PAGE = 20;
		// In a section's header.
		inline constexpr Point<int16_t> GUILD_COUNT = Point<int16_t>(185, 6);
		inline constexpr Point<int16_t> GUILD_PAGE = Point<int16_t>(261, 6);
		inline constexpr Point<int16_t> GUILD_FOLD = Point<int16_t>(172, 8);
		inline constexpr int16_t PAGE_BUTTON = 13;
		// In a member's row.
		inline constexpr int16_t MEMBER_NAME_X = 10;
		inline constexpr int16_t MEMBER_JOB_X = 100;
		inline constexpr int16_t MEMBER_LEVEL_X = 170;
		inline constexpr int16_t MEMBER_TITLE_X = 210;
		inline constexpr int16_t MEMBER_TEXT_Y = 4;
		inline constexpr int16_t MEMBER_JOB_WIDTH = 67;
		inline constexpr int16_t GUILD_ROW_WIDTH = 281;
		// A picked row is filled from its second column (0xFF244768).
		inline constexpr int16_t PICKED_LEFT = 1;
		inline constexpr int16_t PICKED_WIDTH = GUILD_ROW_WIDTH - PICKED_LEFT;
		inline constexpr float PICKED_RED = 0x24 / 255.0f;
		inline constexpr float PICKED_GREEN = 0x47 / 255.0f;
		inline constexpr float PICKED_BLUE = 0x68 / 255.0f;
		// Every guild button shows on the tab; ranks only grey them out.
		// In the order of the guild buttons in Buttons.
		inline const std::pair<const char*, Point<int16_t>> GUILD_BUTTONS[] = {
				{ "BtInvite", Point<int16_t>(9, 316) },
				{ "BtWithdraw", Point<int16_t>(80, 316) },
				{ "BtKick", Point<int16_t>(159, 336) },
				{ "BtUp", Point<int16_t>(9, 336) },
				{ "BtDown", Point<int16_t>(80, 336) },
				{ "BtWhere", Point<int16_t>(159, 316) },
				{ "BtChat", Point<int16_t>(238, 336) },
				{ "BtInfo", Point<int16_t>(238, 363) },
				{ "BtWhisper", Point<int16_t>(238, 316) },
				{ "Btnotice", Point<int16_t>(287, 80) },
				{ "BtPartyInvite", Point<int16_t>(159, 363) },
				{ "BtGuildBBS", Point<int16_t>(80, 363) }
		};
		// The notice prompt takes 100 letters.
		inline constexpr size_t NOTICE_LIMIT = 100;
		// The rank panel sits at the window's right (FUN_0091e2e0) and is
		// laid out as the client draws it (FUN_009165dd): the guild's GP,
		// then a row per rank with its number, its title as edited and its
		// fixed name, greyed while the rank has no title.
		inline constexpr Point<int16_t> GRADE = Point<int16_t>(311, 86);
		// The alliance's rank panel, a pixel higher (FUN_0091e704).
		inline constexpr Point<int16_t> UNION = Point<int16_t>(311, 85);
		inline const std::pair<const char*, Point<int16_t>> GRADE_ART[] = {
				{ "guildgrade3", Point<int16_t>(6, 8) },
				{ "guildgrade4", Point<int16_t>(6, 45) },
				{ "guildgrade0", Point<int16_t>(6, 91) },
				{ "guildgrade1", Point<int16_t>(6, 128) },
				{ "guildgrade2", Point<int16_t>(6, 146) }
		};
		inline constexpr Point<int16_t> GRADE_POINTS = Point<int16_t>(221, 67);
		inline constexpr int16_t GRADE_ROWS = 146;
		inline constexpr int16_t GRADE_ROW_LEFT = 6;
		inline constexpr int16_t GRADE_ROW_WIDTH = 221;
		inline constexpr int16_t GRADE_TEXT_Y = 4;
		inline constexpr int16_t GRADE_RANK_X = 26;
		inline constexpr int16_t GRADE_TITLE_X = 66;
		inline constexpr int16_t GRADE_LABEL_X = 146;
		inline constexpr int16_t GRADE_LABEL_WIDTH = 86;
		inline const char* const GRADE_LABELS[] = { "Master", "Jr.Master", "Member", "Member", "Member" };
		inline constexpr Point<int16_t> GRADE_MODIFY = Point<int16_t>(25, 268);
		inline constexpr Point<int16_t> GRADE_SAVE = Point<int16_t>(125, 268);
		// A guild section's buttons, from BT_GUILD_SECTION.
		enum SectionButton : uint16_t
		{
			FOLD,
			UNFOLD,
			PREVIOUS_PAGE,
			NEXT_PAGE,
			SECTION_BUTTONS
		};

		// `text` cut to `width`, with `tail` after it if cut.
		inline std::string fit(Text::Font font, const std::string& text, int16_t width, const std::string& tail)
		{
			Text measure(font, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);

			if (measure.width() <= width)
				return text;

			std::string cut = text;

			while (!cut.empty())
			{
				cut.pop_back();
				measure.change_text(cut + tail);

				if (measure.width() <= width)
					break;
			}

			return cut + tail;
		}

		inline std::string fraction(size_t part, size_t whole)
		{
			return "[" + std::to_string(part) + "/" + std::to_string(whole) + "]";
		}

		// The client trims what is typed in its prompts.
		inline std::string trim(const std::string& text)
		{
			const char* blanks = " \t\r\n";
			size_t first = text.find_first_not_of(blanks);

			if (first == std::string::npos)
				return "";

			return text.substr(first, text.find_last_not_of(blanks) - first + 1);
		}
	}
}
