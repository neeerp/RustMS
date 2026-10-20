#include "UISpReset.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Data/JobData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Each dialog's size; FUN_004edaef centres it.
		constexpr int16_t WIDTH = 175, HEIGHT = 289, CONFIRM_HEIGHT = 231;
		// The rest is from the dialog's top left (FUN_008b13b2, FUN_008b169a, FUN_008b4373, FUN_008b5a3d). v83
		// draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		// Row r's text baseline; its background, icon and line hang off it.
		constexpr int16_t ROW_TOP = 120, ROW_PITCH = 40;
		constexpr int16_t ROW_X = 7, ICON_X = 9, TEXT_X = 47, BUTTON_X = 131, BY_GAP = 4;
		constexpr size_t NAME_WIDTH = 104;
		const Point<int16_t> CLOSE_AT(155, 6);
		const Point<int16_t> OK_AT(87, 207);
		const Point<int16_t> CANCEL_AT(129, 207);
		// The source dialog's tabs, one per advancement up to the item's (CCtrlTab id 2000 of type 1, report
		// 195b): 170 wide, from Basic.img/Tab2's 4-wide ends and 8-wide junctions.
		constexpr int16_t TABS_X = 5, TABS_Y = 63, TABS_WIDTH = 170, TAB_HEIGHT = 19, TAB_PITCH = 43;
		constexpr int16_t TAB_END = 4, TAB_JUNCTION = 8, TAB_LABEL_Y = 5, TAB_HIT_TOP = 4, TAB_HIT_SLACK = 2;
		// The scrollbar (id 2001) is 155 tall from (153, 99); ours puts its lower arrow's top at the range's end.
		constexpr int16_t SLIDER_X = 153;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(99, 99 + 155 - 13);
		// What an earlier tab may give, under the rows.
		constexpr int16_t SPARE_STRIP_Y = 261, SPARE_RIGHT = 143, SPARE_Y = 269;
		// The confirm dialog: where the level goes, then where it leaves, each under its tab.
		constexpr int16_t TO_Y = 115, FROM_Y = 181, CONFIRM_HEADER_Y = 71, FROM_OFFSET = 66, LINE_Y = 133;

		int16_t row_y(int16_t row)
		{
			return ROW_TOP + ROW_PITCH * row;
		}

		void draw_text(const Text &text, Point<int16_t> at)
		{
			text.draw(at - Point<int16_t>(0, TEXT_RAISE));
		}

		void draw_icon(const Texture &icon, Point<int16_t> at)
		{
			icon.draw(at + icon.get_origin());
		}

		void set(Text &text, const std::string &str)
		{
			if (text.get_text() != str)
				text.change_text(str);
		}

		// Each tab's left and right in the strip (FUN_004de15f): 35 wide on a 43 pitch, unless that is too wide.
		std::vector<Range<int16_t>> tab_spans(int16_t count)
		{
			int16_t width = TAB_PITCH - TAB_JUNCTION;
			int16_t rest = 0;

			if (TAB_PITCH * count > TABS_WIDTH)
			{
				width = (TABS_WIDTH - (count - 1) * TAB_JUNCTION - 2 * TAB_END) / count;
				rest = TABS_WIDTH - (width + 2 * TAB_END) * count;
			}

			std::vector<Range<int16_t>> spans;
			int16_t x = TAB_END;

			for (int16_t i = 0; i < count; i++, rest--)
			{
				int16_t left = x;
				x += width + (rest > 0 ? 1 : 0);
				spans.emplace_back(left, x);
				x += TAB_JUNCTION;
			}

			return spans;
		}

		// The tabs labelled `labels`, `selected` among them (FUN_004dd903). The selected tab looks raised by
		// its pieces alone, and its label is the paler one.
		void draw_tabs(Point<int16_t> at, const std::vector<int32_t> &labels, size_t selected)
		{
			nl::node pieces = NxFiles::ui_classic["Basic.img"]["Tab2"];
			nl::node numerals = NxFiles::ui_classic["UIWindow.img"]["Reset"]["SP"]["Tab"];
			std::vector<Range<int16_t>> spans = tab_spans(static_cast<int16_t>(labels.size()));

			Texture(pieces[selected == 0 ? "left1" : "left0"]).draw(at);

			for (size_t i = 0; i < labels.size(); i++)
			{
				std::string junction;

				if (i + 1 == labels.size())
					junction = i == selected ? "right1" : "right0";
				else
					junction = i == selected ? "middle1" : i + 1 == selected ? "middle2" : "middle0";

				Texture(pieces[junction]).draw(at + Point<int16_t>(spans[i].second(), 0));

				Texture fill = pieces[i == selected ? "fill1" : "fill0"];

				for (int16_t x = spans[i].first(); x < spans[i].second(); x++)
					fill.draw(at + Point<int16_t>(x, 0));

				Texture label = numerals[i == selected ? "disabled" : "enabled"][std::to_string(labels[i])];
				int16_t centre = spans[i].first() + spans[i].length() / 2;
				label.draw(at + Point<int16_t>(centre - label.width() / 2, TAB_LABEL_Y));
			}
		}

		auto reqs_of = [](int32_t skill_id) -> const std::unordered_map<int32_t, int32_t> &
		{
			return SkillData::get(skill_id).get_reqskills();
		};
	}

	UISpReset::UISpReset(int32_t item_id, std::function<void(int32_t, int32_t)> oc)
			: UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT)), onconfirm(std::move(oc)), step(SOURCE),
			  item_tab(sp_reset::item_tab(item_id)), tab(0), first(0), held(false), from(0), to(0)
	{
		path = sp_reset::job_path(Stage::get().get_player().get_stats().get_stat(MapleStat::Id::JOB));

		spare = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);
		less = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLUE);
		less_by = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLUE, "(-1)");
		more = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::RED);
		more_by = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::RED, "(+1)");

		slider = Slider(Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, 0, [&](bool upwards)
		{
			first += upwards ? -1 : 1;
			refresh();
		});

		show(SOURCE);
	}

	void UISpReset::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		nl::node sp = NxFiles::ui_classic["UIWindow.img"]["Reset"]["SP"];

		if (step == CONFIRM)
		{
			// FUN_008b64c6: a lone tab naming the skill's advancement.
			auto header = [&](int32_t skill_id, int16_t y)
			{
				int32_t job = skill_id / 10000;

				draw_tabs(position + Point<int16_t>(TABS_X, y), { job % 100 == 0 ? 0 : job % 10 + 1 }, 0);
			};

			auto row = [&](int32_t skill_id, const Text &name, const Text &value, const Text &by, int16_t y)
			{
				draw_icon(SkillData::get(skill_id).get_icon(SkillData::Icon::NORMAL),
						  position + Point<int16_t>(ICON_X, y - 19));
				draw_text(name, position + Point<int16_t>(TEXT_X, y - 18));
				draw_text(value, position + Point<int16_t>(TEXT_X, y));
				draw_text(by, position + Point<int16_t>(TEXT_X + value.width() + BY_GAP, y));
			};

			header(to, CONFIRM_HEADER_Y);
			row(to, to_name, more, more_by, TO_Y);
			header(from, CONFIRM_HEADER_Y + FROM_OFFSET);
			row(from, from_name, less, less_by, FROM_Y);

			Texture line = sp["line"];
			line.draw(position + Point<int16_t>(ROW_X, LINE_Y));
			line.draw(position + Point<int16_t>(ROW_X + 20, LINE_Y));
		} else
		{
			if (step == SOURCE)
			{
				std::vector<int32_t> labels;

				for (int32_t t = 0; t <= item_tab; t++)
					labels.push_back(t);

				draw_tabs(position + Point<int16_t>(TABS_X, TABS_Y), labels, tab);

				if (tab < item_tab)
				{
					Texture(sp["backgrnd1"]).draw(position + Point<int16_t>(0, SPARE_STRIP_Y));
					draw_text(spare, position + Point<int16_t>(SPARE_RIGHT, SPARE_Y));
				}
			} else
			{
				// FUN_008b4e13: the item's advancement alone.
				draw_tabs(position + Point<int16_t>(TABS_X, TABS_Y), { item_tab }, 0);
			}

			Texture shown = sp["skill1"], greyed = sp["skill0"], line = sp["line"];

			for (int16_t r = 0; r < ROWS; r++)
			{
				size_t i = first + r;

				if (i >= entries.size())
					break;

				const Entry &entry = entries[i];
				int16_t y = row_y(r);
				const SkillData &data = SkillData::get(entry.skill_id);

				(entry.enabled ? shown : greyed).draw(position + Point<int16_t>(ROW_X, y - 21));
				draw_icon(data.get_icon(entry.enabled ? SkillData::Icon::NORMAL : SkillData::Icon::DISABLED),
						  position + Point<int16_t>(ICON_X, y - 19));
				draw_text(entry.name, position + Point<int16_t>(TEXT_X, y - 18));

				if (step == TARGET && entry.skill_id == from)
				{
					draw_text(less, position + Point<int16_t>(TEXT_X, y));
					draw_text(less_by, position + Point<int16_t>(TEXT_X + less.width() + BY_GAP, y));
				} else
				{
					draw_text(entry.level, position + Point<int16_t>(TEXT_X, y));
				}

				line.draw(position + Point<int16_t>(ROW_X, y + 16));
			}

			slider.draw(position);
		}

		UIElement::draw_buttons(alpha);
	}

	void UISpReset::update()
	{
		UIElement::update();

		// The levels are read each time v83 draws; which rows may be picked is not.
		const SkillBook &skills = Stage::get().get_player().get_skills();

		for (Entry &entry : entries)
			set(entry.level, std::to_string(skills.get_level(entry.skill_id)));

		set(less, std::to_string(skills.get_level(from) - 1));
		set(more, std::to_string(skills.get_level(to) + 1));

		if (step == SOURCE)
			set(spare, std::to_string(sp_reset::spare(character(), item_tab)));
	}

	void UISpReset::update_screen(int16_t new_width, int16_t new_height)
	{
		int16_t x = std::clamp<int16_t>(new_width / 2 - dimension.x() / 2, 0,
										std::max<int16_t>(new_width - dimension.x(), 0));
		int16_t y = std::clamp<int16_t>(new_height / 2 - dimension.y() / 2, 0,
										std::max<int16_t>(new_height - dimension.y(), 0));

		position = Point<int16_t>(x, y);
	}

	void UISpReset::remove_cursor()
	{
		UIElement::remove_cursor();

		held = false;
		slider.remove_cursor();
	}

	Cursor::State UISpReset::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool press = clicked && !held;
		held = clicked;

		Point<int16_t> cursor = cursorpos - position;

		if (step != CONFIRM)
		{
			Cursor::State state = slider.send_cursor(cursor, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		// A tab is picked as the button goes down, even the one already picked (FUN_004dd822), which lists
		// its skills again (FUN_008b23ed).
		Point<int16_t> strip = cursor - Point<int16_t>(TABS_X, TABS_Y);

		if (step == SOURCE && press && strip.y() >= TAB_HIT_TOP && strip.y() < TAB_HEIGHT)
		{
			std::vector<Range<int16_t>> spans = tab_spans(static_cast<int16_t>(item_tab + 1));

			for (int32_t t = 0; t <= item_tab; t++)
			{
				if (strip.x() >= spans[t].first() - TAB_HIT_SLACK && strip.x() <= spans[t].second() + TAB_HIT_SLACK)
				{
					Sound(Sound::Name::TAB).play();
					tab = t;
					list(path[tab]);

					return Cursor::State::CLICKING;
				}
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UISpReset::send_scroll(double yoffset)
	{
		if (step != CONFIRM)
			slider.send_scroll(yoffset);
	}

	void UISpReset::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// Only the confirm dialog has an OK for Enter (FUN_004edd21).
		if (escape)
			deactivate();
		else if (keycode == KeyAction::Id::RETURN && step == CONFIRM)
			confirm();
	}

	UIElement::Type UISpReset::get_type() const
	{
		return TYPE;
	}

	Button::State UISpReset::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::CLOSE || buttonid == Buttons::CANCEL)
		{
			deactivate();
		} else if (buttonid == Buttons::OK)
		{
			confirm();
		} else if (buttonid < Buttons::TARGET_ROW)
		{
			from = entries[first + buttonid - Buttons::SOURCE_ROW].skill_id;
			show(TARGET);
		} else if (buttonid < Buttons::CLOSE)
		{
			to = entries[first + buttonid - Buttons::TARGET_ROW].skill_id;
			show(CONFIRM);
		}

		return Button::State::NORMAL;
	}

	void UISpReset::show(Step next)
	{
		step = next;

		nl::node reset = NxFiles::ui_classic["UIWindow.img"]["Reset"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		sprites.clear();
		buttons.clear();

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_AT);

		if (step == CONFIRM)
		{
			dimension = Point<int16_t>(WIDTH, CONFIRM_HEIGHT);
			sprites.emplace_back(reset["SP"]["backgrnd3"]);
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);

			auto name = [](int32_t skill_id)
			{
				Text text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						  SkillData::get(skill_id).get_name());
				string_format::format_with_ellipsis(text, NAME_WIDTH);

				return text;
			};

			to_name = name(to);
			from_name = name(from);
		} else
		{
			bool source = step == SOURCE;

			dimension = Point<int16_t>(WIDTH, HEIGHT);
			sprites.emplace_back(reset["SP"][source ? "backgrnd" : "backgrnd2"]);

			for (int16_t r = 0; r < ROWS; r++)
			{
				uint16_t id = (source ? Buttons::SOURCE_ROW : Buttons::TARGET_ROW) + r;
				nl::node look = reset["Common"][source ? "BtDown" : "BtUP"];

				buttons[id] = std::make_unique<MapleButton>(look, Point<int16_t>(BUTTON_X, row_y(r) - 1));
			}

			// The source dialog opens on the first tab; the target lists only the item's advancement.
			tab = source ? 0 : item_tab;
			first = 0;
			list(path[tab]);
		}

		update();
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UISpReset::list(int32_t job)
	{
		const SkillBook &skills = Stage::get().get_player().get_skills();

		entries.clear();

		for (int32_t skill_id : JobData::get(job).get_skills())
		{
			const SkillData &data = SkillData::get(skill_id);

			if (data.is_invisible() && skills.get_masterlevel(skill_id) == 0)
				continue;

			Text name(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, data.get_name());
			string_format::format_with_ellipsis(name, NAME_WIDTH);

			entries.push_back({ skill_id, name, Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK),
								false });
		}

		// Another tab keeps the scroll, as far as its list allows (FUN_004d7dac).
		auto count = static_cast<int16_t>(entries.size());
		first = std::clamp<int16_t>(first, 0, std::max<int16_t>(count - ROWS, 0));
		slider.setrows(first, ROWS, count);
		refresh();
	}

	void UISpReset::refresh()
	{
		const SkillBook &skills = Stage::get().get_player().get_skills();
		sp_reset::Character c = character();

		for (int16_t r = 0; r < ROWS; r++)
		{
			size_t i = first + r;
			uint16_t id = (step == SOURCE ? Buttons::SOURCE_ROW : Buttons::TARGET_ROW) + r;
			Button &button = *buttons[id];

			if (i >= entries.size())
			{
				button.set_active(false);
				continue;
			}

			Entry &entry = entries[i];
			int32_t skill_id = entry.skill_id;

			if (step == SOURCE)
			{
				entry.enabled = sp_reset::may_take(c, skill_id, tab, item_tab, reqs_of);
			} else
			{
				const SkillData &data = SkillData::get(skill_id);
				entry.enabled = sp_reset::may_give(c, skill_id, from, skills.get_masterlevel(skill_id),
												   data.get_masterlevel(), data.get_reqskills());
			}

			button.set_active(true);
			button.set_state(entry.enabled ? Button::State::NORMAL : Button::State::DISABLED);
		}
	}

	void UISpReset::confirm()
	{
		deactivate();
		onconfirm(to, from);
	}

	sp_reset::Character UISpReset::character() const
	{
		Player &player = Stage::get().get_player();
		const CharStats &stats = player.get_stats();

		return { stats.get_stat(MapleStat::Id::JOB), stats.get_stat(MapleStat::Id::LEVEL),
				 stats.get_stat(MapleStat::Id::SP), player.get_skills().collect_levels() };
	}
}
