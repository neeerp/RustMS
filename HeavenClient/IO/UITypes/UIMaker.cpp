#include "UIMaker.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Character/Inventory/ItemDrop.h"
#include "../../Data/EquipData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (FUN_008229b8, FUN_00824c82; report c006b).
		constexpr Rectangle<int16_t> RESULT = Rectangle<int16_t>(15, 67, 39, 91);
		constexpr int16_t SLOT_SIZE = 37;
		constexpr Point<int16_t> CATEGORY = Point<int16_t>(77, 71);
		constexpr Point<int16_t> ITEM = Point<int16_t>(157, 71);
		constexpr int16_t CATEGORY_WIDTH = 78;
		constexpr int16_t ITEM_WIDTH = 143;
		constexpr Rectangle<int16_t> INGREDIENTS = Rectangle<int16_t>(11, 300, 124, 220);
		constexpr Point<int16_t> FEE = Point<int16_t>(277, 226);
		constexpr Point<int16_t> STIMULATOR = Point<int16_t>(19, 269);
		// Where a stimulator and gems are dropped (FUN_00822350's +0xce8 and +0xcf8).
		constexpr Rectangle<int16_t> STIMULATOR_DROP = Rectangle<int16_t>(11, 80, 250, 313);
		constexpr Rectangle<int16_t> GEM_DROP = Rectangle<int16_t>(80, 304, 250, 313);
		constexpr Point<int16_t> BAR = Point<int16_t>(6, 320);
		constexpr int16_t DRAG_HEIGHT = 20;

		// The gauge's cover is drawn a column at a time from 8 plus its progress to 301, which grows 58 a second
		// (FUN_00825b50).
		constexpr int16_t COVER_FROM = 8;
		constexpr int16_t COVER_TO = 302;
		constexpr int32_t GAUGE_SPEED = 58;

		// Ingredient slot i: three a row, 93 apart, rows 50 apart from 130.
		Point<int16_t> slot_position(size_t i)
		{
			return Point<int16_t>(static_cast<int16_t>(i % 3 * 93 + 19), static_cast<int16_t>(i < 3 ? 130 : 180));
		}

		Point<int16_t> gem_position(int32_t k)
		{
			return Point<int16_t>(static_cast<int16_t>(112 + 49 * k), 269);
		}

		// An icon centred on the square at `at` of side `side`.
		void draw_centred(const Texture &icon, Point<int16_t> at, int16_t side, float opacity)
		{
			Point<int16_t> size = icon.get_dimensions();
			Point<int16_t> place = at + Point<int16_t>((side - size.x()) / 2, (side - size.y()) / 2) + icon.get_origin();

			icon.draw(DrawArgument(place, opacity));
		}

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		// StringPool 0x2e1, 0x2e4, 0x2e5, 0x2ed, 0x2f8.
		constexpr const char *SELECT_FIRST = "Please select the item you wish to make first!";
		constexpr const char *NO_MORE = "You cannot apply any more ingredients.";
		constexpr const char *NOT_CRYSTAL = "You cannot use this item as an ingredient to make the Monster Crystal.";
		constexpr const char *CRYSTAL_HINT = "Apply the drop items here\\n to make the Monster Crystal.";
		// StringPool 0x2f9, 0x11cf.
		constexpr const char *DISASSEMBLY_HINT = "Apply the equipment to be disassembled here\\n to acquire the Monster Crystal.";
		constexpr const char *NOT_DISASSEMBLABLE = "This item cannot be disassembled.";

		// The recipe for `item_id`, whoever may make it, or none.
		const maker::Recipe *recipe_of(int32_t item_id, const std::vector<maker::Recipe> &all)
		{
			for (const maker::Recipe &r : all)
				if (r.item_id == item_id)
					return &r;

			return nullptr;
		}

		Rectangle<int16_t> square(Point<int16_t> at)
		{
			return Rectangle<int16_t>(at, at + Point<int16_t>(SLOT_SIZE, SLOT_SIZE));
		}

		// The drop lists' boxes are 18 high, their rows 16, and ten show (FUN_00822350, FUN_004c7068).
		constexpr int16_t BOX_HEIGHT = 18;
		constexpr int16_t ROW = 16;
		constexpr size_t SHOWN_ROWS = 10;
		// v83 draws the drop lists' text from its top; HC's text sits this much lower for the same point.
		constexpr int16_t TEXT_DROP = 7;
	}

	void UIMaker::DropList::init(Point<int16_t> p, int16_t w)
	{
		position = p;
		width = w;

		nl::node src = NxFiles::ui_classic["Basic.img"]["ComboBox4"];
		// By Button::State, IDENTITY having none.
		const char *states[] = { "normal", "disabled", "mouseOver", "pressed" };

		for (size_t state = 0; state < std::size(states); state++)
			for (size_t piece = 0; piece < 3; piece++)
				pieces[state][piece] = src[states[state]][piece];

		shown = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		// The scroll bar's right edge is the list's.
		int16_t top = position.y() + BOX_HEIGHT - 1;
		slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(top + 1, top + SHOWN_ROWS * ROW), position.x() + width - 15,
						SHOWN_ROWS, 1, [this](bool up) { scroll(up ? -1 : 1); });
	}

	void UIMaker::DropList::set(std::vector<Choice> c)
	{
		choices = std::move(c);
		rows.clear();
		lit_rows.clear();

		// FUN_004c3a0d ellipsises a row past the list's width, less the scroll bar's (I: less the text's indent).
		int16_t room = (choices.size() > SHOWN_ROWS ? width - 18 : width) - 7 - 2;

		for (const Choice &choice : choices)
		{
			rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, choice.text);
			lit_rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, choice.text);
			string_format::format_with_ellipsis(rows.back(), room);
			string_format::format_with_ellipsis(lit_rows.back(), room);
		}

		chosen = 0;
		first = 0;
		hovered = -1;
		open = false;
		shown.change_text(choices.empty() ? "" : choices[0].text);
		string_format::format_with_ellipsis(shown, width - 17 - 3);
	}

	size_t UIMaker::DropList::shown_rows() const
	{
		return std::min(SHOWN_ROWS, rows.size());
	}

	int16_t UIMaker::DropList::list_height() const
	{
		return static_cast<int16_t>(shown_rows() * ROW + 1);
	}

	int16_t UIMaker::DropList::row_width() const
	{
		return rows.size() > SHOWN_ROWS ? width - 15 : width - 2;
	}

	void UIMaker::DropList::draw(Point<int16_t> parent) const
	{
		Point<int16_t> at = parent + position;
		Button::State state = !enabled ? Button::State::DISABLED : open ? Button::State::PRESSED :
							  over_box ? Button::State::MOUSEOVER : Button::State::NORMAL;
		const Texture &left = pieces[state][0];
		const Texture &middle = pieces[state][1];
		const Texture &right = pieces[state][2];

		// FUN_004c5fb9: the left cap, as many middles as fit, then the arrow.
		Point<int16_t> piece = at;
		left.draw(piece);
		piece.shift_x(left.width());

		if (int16_t mid = middle.width(); mid > 0)
		{
			for (int16_t n = (width - left.width() - right.width() - 1) / mid; n > 0; n--)
			{
				middle.draw(piece);
				piece.shift_x(mid);
			}
		}

		right.draw(piece);
		shown.draw(at + Point<int16_t>(3, 3 - TEXT_DROP));

		if (!open)
			return;

		// FUN_004c3a0d: a grey border round a light list, the hovered row barred with its text in white.
		Point<int16_t> list = at + Point<int16_t>(0, BOX_HEIGHT - 1);
		int16_t height = list_height();

		ColorBox(width, height, Color::Name::DUSTYGRAY, 1.0f).draw(list);
		ColorBox(width - 3, height - 2, Color::Name::COMBOLIST, 1.0f).draw(list + Point<int16_t>(1, 1));

		for (size_t i = 0; i < shown_rows(); i++)
		{
			size_t row = first + i;
			Point<int16_t> top = list + Point<int16_t>(0, static_cast<int16_t>(i * ROW));
			bool lit = static_cast<int32_t>(row) == hovered;

			if (lit)
				ColorBox(row_width(), ROW - 1, Color::Name::COMBOHOVER, 1.0f).draw(top + Point<int16_t>(1, 1));

			(lit ? lit_rows : rows)[row].draw(top + Point<int16_t>(7, 3 - TEXT_DROP));
		}

		if (rows.size() > SHOWN_ROWS)
			slider.draw(parent);
	}

	bool UIMaker::DropList::in_box(Point<int16_t> cursor) const
	{
		return Rectangle<int16_t>(position, position + Point<int16_t>(width, BOX_HEIGHT)).contains(cursor);
	}

	bool UIMaker::DropList::in_list(Point<int16_t> cursor) const
	{
		if (!open)
			return false;

		Point<int16_t> lt = position + Point<int16_t>(0, BOX_HEIGHT - 1);

		return Rectangle<int16_t>(lt, lt + Point<int16_t>(width, list_height())).contains(cursor);
	}

	int32_t UIMaker::DropList::row_at(Point<int16_t> cursor) const
	{
		if (!in_list(cursor) || cursor.x() >= position.x() + 1 + row_width())
			return -1;

		int16_t y = cursor.y() - (position.y() + BOX_HEIGHT - 1) - 1;

		if (y < 0)
			return -1;

		size_t row = first + y / ROW;

		return row < std::min(first + SHOWN_ROWS, rows.size()) ? static_cast<int32_t>(row) : -1;
	}

	bool UIMaker::DropList::hover(Point<int16_t> cursor, bool clicked)
	{
		over_box = in_box(cursor);

		if (open && rows.size() > SHOWN_ROWS && slider.send_cursor(cursor, clicked) != Cursor::State::IDLE)
		{
			hovered = -1;

			return true;
		}

		if (open)
			hovered = row_at(cursor);

		return false;
	}

	void UIMaker::DropList::leave()
	{
		over_box = false;
		hovered = -1;
		slider.remove_cursor();
	}

	void UIMaker::DropList::scroll(int32_t by)
	{
		if (!open || rows.size() <= SHOWN_ROWS)
			return;

		int32_t last = static_cast<int32_t>(rows.size() - SHOWN_ROWS);
		first = static_cast<size_t>(std::clamp(static_cast<int32_t>(first) + by, 0, last));
		slider.setrows(static_cast<int16_t>(first), static_cast<int16_t>(SHOWN_ROWS),
					   static_cast<int16_t>(rows.size()));
	}

	void UIMaker::DropList::toggle()
	{
		if (!enabled || choices.empty())
			return;

		open = !open;
		hovered = -1;

		// The list opens on what is chosen.
		if (open)
		{
			first = rows.size() > SHOWN_ROWS ? std::min(chosen, rows.size() - SHOWN_ROWS) : 0;
			slider.setrows(static_cast<int16_t>(first), static_cast<int16_t>(SHOWN_ROWS),
						   static_cast<int16_t>(rows.size()));
		}
	}

	void UIMaker::DropList::close()
	{
		open = false;
		hovered = -1;
		slider.remove_cursor();
	}

	void UIMaker::DropList::select(size_t index)
	{
		if (index >= choices.size())
			return;

		chosen = index;
		shown.change_text(choices[index].text);
		string_format::format_with_ellipsis(shown, width - 17 - 3);
		close();
	}

	bool UIMaker::DropList::is_open() const
	{
		return open;
	}

	size_t UIMaker::DropList::selected() const
	{
		return chosen;
	}

	int32_t UIMaker::DropList::data() const
	{
		return chosen < choices.size() ? choices[chosen].data : 0;
	}

	void UIMaker::DropList::enable(bool on)
	{
		enabled = on;

		if (!on)
			close();
	}

	const std::vector<maker::Recipe> &UIMaker::recipes()
	{
		static const std::vector<maker::Recipe> all = []()
		{
			std::vector<maker::Recipe> loaded;

			for (nl::node group : nl::nx::etc["ItemMake.img"])
			{
				for (nl::node node : group)
				{
					maker::Recipe recipe;
					recipe.item_id = string_conversion::or_zero<int32_t>(node.name());
					recipe.group = string_conversion::or_zero<int32_t>(group.name());
					recipe.count = std::max<int32_t>(node["itemNum"].get_integer(), 1);
					recipe.meso = node["meso"];
					recipe.catalyst = node["catalyst"];
					recipe.req_level = node["reqLevel"];
					recipe.req_skill_level = node["reqSkillLevel"];
					recipe.req_item = node["reqItem"];
					recipe.req_equip = node["reqEquip"];
					recipe.gem_slots = node["tuc"];

					for (nl::node ingredient : node["recipe"])
						recipe.ingredients.emplace_back(ingredient["item"], ingredient["count"]);

					for (nl::node quest : node["reqQuest"])
						recipe.req_quests.emplace_back(string_conversion::or_zero<int32_t>(quest.name()),
													   quest.get_integer());

					if (recipe.item_id)
						loaded.push_back(recipe);
				}
			}

			return loaded;
		}();

		return all;
	}

	int32_t UIMaker::crystal_of(int32_t item_id)
	{
		static const std::vector<maker::CrystalLevels> crystals = []()
		{
			std::vector<maker::CrystalLevels> levels;

			for (nl::node crystal : nl::nx::item["Etc"]["0426.img"])
			{
				nl::node info = crystal["info"];

				if (info["lvMin"] && info["lvMax"])
					levels.push_back({ string_conversion::or_zero<int32_t>(crystal.name()), info["lvMin"],
									   info["lvMax"] });
			}

			return levels;
		}();

		// FUN_005e8bff: an etc drop with a level some crystal is made from.
		if (item_id / 10000 != 400)
			return 0;

		int32_t level = nl::nx::item["Etc"]["0400.img"][string_format::extend_id(item_id, 8)]["info"]["lv"];

		return maker::crystal_for(level, crystals);
	}

	UIMaker::UIMaker() : UIElement(Point<int16_t>(), Point<int16_t>()), mode(0), recipe(nullptr), stimulated(false),
						 stimulator(0), equip_slot(0), state(State::IDLE),
						 elapsed(0), was_clicked(false), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Maker"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		not_use = src["notUse"];
		random_make = src["randomMake"];
		random_recipe = src["randomRecipe"];
		bar = src["GaugeBar"]["bar"];
		gauge = src["GaugeBar"]["gauge"];

		buttons[Buttons::START] = std::make_unique<MapleButton>(src["BtStart"], Point<int16_t>(82, 356));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(163, 356));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"],
															   Point<int16_t>(292, 6));
		buttons[Buttons::START]->set_state(Button::State::DISABLED);
		working = Sound(nl::nx::sound["UI.img"]["ItemMake"]);

		// The two fonts of FUN_00822350: counts light blue-grey, the fee dark blue.
		count = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::MAKERCOUNT);
		fee = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::MAKERFEE);

		category.init(CATEGORY, CATEGORY_WIDTH);
		item.init(ITEM, ITEM_WIDTH);

		// FUN_008232bf: what the player's job and Maker may make.
		Player &player = Stage::get().get_player();
		int16_t job = static_cast<int16_t>(player.get_stats().get_stat(MapleStat::Id::JOB));
		std::vector<int32_t> groups = maker::groups(job);

		facts.level = player.get_level();
		facts.maker_level = player.get_skills().get_level(maker::skill_id(job));
		facts.holds = [&player](int32_t id) { return player.get_inventory().get_total_item_count(id) > 0; };
		facts.wears = [&player](int32_t id)
		{
			return player.get_inventory().find_item(InventoryType::Id::EQUIPPED, id) != 0;
		};
		facts.quest_state = [&player](int32_t quest)
		{
			QuestLog &quests = player.get_quests();
			int16_t id = static_cast<int16_t>(quest);

			return quests.is_completed(id) ? 2 : quests.is_started(id) ? 1 : 0;
		};

		for (const maker::Recipe &r : recipes())
			if (std::find(groups.begin(), groups.end(), r.group) != groups.end() && maker::listed(r, facts))
				listed.push_back(&r);

		// FUN_008234a3.
		std::vector<DropList::Choice> categories;

		for (int32_t key : maker::categories(listed))
		{
			std::string name;

			switch (key)
			{
				case 0:
					name = "Category";
					break;
				case maker::DISASSEMBLE:
					name = "Disassemble Equipment";
					break;
				case maker::CRYSTAL:
					name = "Monster Crystal";
					break;
				case maker::GEMS:
					name = "Strengthening Gem";
					break;
				case maker::USE:
					name = "Use";
					break;
				case maker::SETUP:
					name = "Setup";
					break;
				case maker::ETC:
					name = "Etc";
					break;
				case maker::RANDOM:
					name = "???";
					break;
				default:
				{
					// Named for the first recipe's item (FUN_00823039).
					std::vector<const maker::Recipe *> under = maker::under(key, listed);
					name = under.empty() ? "" : maker::equip_category(under.front()->item_id);
					break;
				}
			}

			categories.push_back({ name, key });
		}

		category.set(categories);
		item.set({});
		item.enable(false);

		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	UIMaker::~UIMaker()
	{
		working.stop();
	}

	void UIMaker::draw(float alpha) const
	{
		UIElement::draw(alpha);

		// The result: the item, or for the rest the random make's art (FUN_00824d81).
		if (mode == 1 && recipe)
			draw_centred(ItemData::get(recipe->item_id).get_icon(false), position + Point<int16_t>(15, 39), 52, 1.0f);
		else if (mode > 1)
			draw_centred(random_make, position + Point<int16_t>(15, 39), 52, 1.0f);

		// The ingredients (FUN_00825250): faded when there are too few, with their counts; past them, or with
		// nothing picked, unused.
		for (size_t i = 0; i < 6; i++)
		{
			Point<int16_t> at = position + slot_position(i);

			if (i >= slots.size())
			{
				not_use.draw(at);
				continue;
			}

			const Slot &slot = slots[i];

			if (slot.item_id)
				draw_centred(ItemData::get(slot.item_id).get_icon(false), at, SLOT_SIZE,
							 slot.owned >= slot.needed ? 1.0f : 100.0f / 255.0f);
			else
				random_recipe.draw(at + Point<int16_t>((SLOT_SIZE - random_recipe.width()) / 2,
													   (SLOT_SIZE - random_recipe.height()) / 2) +
								   random_recipe.get_origin());

			Text line = count;
			line.change_text(mode == 2 ? std::to_string(slot.owned) + "/???" :
							 maker::count_text(slot.owned, slot.needed));
			line.draw(at + Point<int16_t>(40, 37 - line.height()));
		}

		// The gems and the stimulator (FUN_008257b9, FUN_008259a2): unused past the recipe's slots, and the
		// stimulator's icon faded until one is applied.
		int32_t open = gem_slots();

		for (int32_t k = 0; k < 4; k++)
		{
			if (k < static_cast<int32_t>(gems.size()))
				draw_centred(ItemData::get(gems[k]).get_icon(false), position + gem_position(k), SLOT_SIZE, 1.0f);
			else if (k >= open)
				not_use.draw(position + gem_position(k));
		}

		if (mode == 1 && recipe && recipe->catalyst)
			draw_centred(ItemData::get(recipe->catalyst).get_icon(false), position + STIMULATOR, SLOT_SIZE,
						 stimulated ? 1.0f : 100.0f / 255.0f);
		else if (mode == 2 && stimulated)
			draw_centred(ItemData::get(stimulator).get_icon(false), position + STIMULATOR, SLOT_SIZE, 1.0f);
		else if (mode != 2)
			not_use.draw(position + STIMULATOR);

		// The fee (FUN_0082507e), for a recipe, a crystal or a disassembly.
		if ((mode == 1 && recipe) || mode == 3 || (mode == 4 && !slots.empty() && slots[0].item_id))
		{
			Text line = fee;
			std::string text = std::to_string(current_fee());
			string_format::split_number(text);
			line.change_text(text);
			line.draw(position + FEE);
		}

		// The gauge: its cover is lifted from the left as it fills (FUN_00825b50).
		bar.draw(position + BAR);

		int16_t progress = state == State::FILLING ? static_cast<int16_t>(GAUGE_SPEED * elapsed / 1000) : 0;

		for (int16_t x = COVER_FROM + progress; x < COVER_TO; x++)
			gauge.draw(position + Point<int16_t>(x, BAR.y()));

		category.draw(position);
		item.draw(position);
	}

	void UIMaker::update()
	{
		UIElement::update();

		// FUN_00824a06: the counts and Start are checked every frame.
		if (state == State::IDLE || state == State::READY)
			recount();

		if (state != State::FILLING)
			return;

		elapsed += Constants::TIMESTEP;

		if (elapsed >= maker::GAUGE_MS)
			send();
	}

	void UIMaker::remove_cursor()
	{
		UIElement::remove_cursor();

		was_clicked = false;
		dragged = false;
		category.leave();
		item.leave();
		clear_tooltip();
	}

	Cursor::State UIMaker::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool click = clicked && !was_clicked;
		bool release = !clicked && was_clicked;
		was_clicked = clicked;

		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;

		// An open list takes the cursor: a row is chosen when the button comes up on it, and a press anywhere
		// else closes it (FUN_004c4121, FUN_004c4010).
		for (DropList *list : { &category, &item })
		{
			bool scrolling = list->hover(cursor, clicked);

			if (!list->is_open())
				continue;

			clear_tooltip();

			if (scrolling)
				return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;

			int32_t row = list->row_at(cursor);

			if (release && row >= 0)
			{
				size_t before = list->selected();
				list->select(static_cast<size_t>(row));

				if (list->selected() != before)
				{
					if (list == &category)
						pick_category();
					else
						pick_item();
				}

				return Cursor::State::IDLE;
			}

			if (click && !list->in_list(cursor))
			{
				list->close();

				return Cursor::State::IDLE;
			}

			return list->in_list(cursor) ? Cursor::State::CANCLICK : Cursor::State::IDLE;
		}

		bool idle = state == State::IDLE || state == State::READY;

		for (DropList *list : { &category, &item })
		{
			if (list->in_box(cursor))
			{
				clear_tooltip();

				if (click && idle)
				{
					list->toggle();
					Sound(Sound::Name::BUTTONCLICK).play();
				}

				return Cursor::State::CANCLICK;
			}
		}

		// A press on a gem or the stimulator takes it back out (FUN_008269e9, FUN_00826a5c).
		if (click && idle && (mode == 1 || mode == 2))
		{
			for (size_t k = 0; k < gems.size(); k++)
			{
				if (square(gem_position(static_cast<int32_t>(k))).contains(cursor))
				{
					gems.erase(gems.begin() + static_cast<std::ptrdiff_t>(k));
					clear_tooltip();
					Sound(Sound::Name::DRAGEND).play();

					return Cursor::State::CLICKING;
				}
			}

			if (stimulated && square(STIMULATOR).contains(cursor))
			{
				stimulated = false;
				stimulator = 0;
				clear_tooltip();
				Sound(Sound::Name::DRAGEND).play();

				return Cursor::State::CLICKING;
			}
		}

		// A press on a dropped-in ingredient takes it back out (FUN_008266f1, FUN_00826838).
		if (click && idle && mode > 1)
		{
			int32_t i = slot_at(cursor);

			if (i >= 0 && slots[i].item_id)
			{
				slots[i].item_id = 0;
				clear_tooltip();
				Sound(Sound::Name::DRAGEND).play();
				recount();

				return Cursor::State::CLICKING;
			}
		}

		show_tooltip(cursor);

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		if (click && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIMaker::send_scroll(double yoffset)
	{
		for (DropList *list : { &category, &item })
			if (list->is_open())
				list->scroll(yoffset > 0 ? -1 : 1);
	}

	void UIMaker::send_key(int32_t, bool pressed, bool escape)
	{
		if (!pressed || !escape)
			return;

		if (category.is_open() || item.is_open())
		{
			category.close();
			item.close();
		}
		else
		{
			close();
		}
	}

	bool UIMaker::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		auto [tab, s] = icon.get_source();

		if (tab == InventoryType::Id::NONE || tab == InventoryType::Id::EQUIPPED)
			return true;

		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = inventory.get_item_id(tab, s);

		if (id == 0)
			return true;

		// FUN_004f3dbd, before the Maker sees it (0x4f3e57).
		if (inventory.is_sealed(tab, s))
		{
			notice(item_drop::SEALED);

			return true;
		}

		// FUN_00825d15: only while nothing is under way.
		if (state != State::IDLE && state != State::READY)
			return true;

		if (mode == 0)
		{
			notice(SELECT_FIRST);

			return true;
		}

		Point<int16_t> cursor = cursorpos - position;

		// A recipe's ingredients are its own; dropped ones are the crystal's drops or the equipment to take
		// apart (FUN_00825dd2).
		if (STIMULATOR_DROP.contains(cursor))
			drop_stimulator(id);
		else if (GEM_DROP.contains(cursor))
			drop_gem(id);
		else if (INGREDIENTS.contains(cursor) && mode == 4)
			drop_equip(id, tab == InventoryType::Id::EQUIP ? s : 0);
		else if (INGREDIENTS.contains(cursor) && mode == 3)
			drop_crystal(id);
		else if (INGREDIENTS.contains(cursor) && mode == 2)
			drop_random(id);

		return true;
	}

	void UIMaker::drop_crystal(int32_t id)
	{
		// One slot, so a filled one refuses before the same item would (FUN_00825dd2).
		if (!slots.empty() && slots[0].item_id)
		{
			notice(NO_MORE);

			return;
		}

		if (!crystal_of(id))
		{
			notice(NOT_CRYSTAL);

			return;
		}

		slots[0].item_id = id;
		Sound(Sound::Name::DRAGEND).play();
		recount();
	}

	void UIMaker::drop_equip(int32_t id, int16_t slot)
	{
		if (!slots.empty() && slots[0].item_id)
		{
			notice(NO_MORE);

			return;
		}

		const maker::Recipe *made = recipe_of(id, recipes());

		if (slot == 0 || !made || !maker::disassemblable(*made))
		{
			notice(NOT_DISASSEMBLABLE);

			return;
		}

		slots[0].item_id = id;
		equip_slot = slot;
		Sound(Sound::Name::DRAGEND).play();
		recount();
	}

	void UIMaker::drop_random(int32_t id)
	{
		std::vector<int32_t> applied;

		for (const Slot &slot : slots)
			if (slot.item_id)
				applied.push_back(slot.item_id);

		std::string refusal = maker::random_ingredient_refusal(applied, id);

		if (!refusal.empty())
		{
			notice(refusal);

			return;
		}

		int32_t owned = Stage::get().get_player().get_inventory().get_total_item_count(id);

		// 0x2f3: 1 to what the player has.
		UI::get().emplace<UIEnterNumber>("How many would you like to apply?", [id](int32_t quantity)
		{
			auto window = UI::get().get_element<UIMaker>();

			if (window && window->is_active())
				window->apply_random(id, quantity);
		}, owned, 1);
	}

	void UIMaker::apply_random(int32_t id, int32_t quantity)
	{
		// The window may have changed while it asked.
		if (mode != 2 || (state != State::IDLE && state != State::READY))
			return;

		std::vector<int32_t> applied;

		for (const Slot &slot : slots)
			if (slot.item_id)
				applied.push_back(slot.item_id);

		if (!maker::random_ingredient_refusal(applied, id).empty())
			return;

		for (Slot &slot : slots)
		{
			if (slot.item_id)
				continue;

			slot.item_id = id;
			slot.owned = quantity;
			Sound(Sound::Name::DRAGEND).play();
			recount();

			return;
		}
	}

	void UIMaker::drop_gem(int32_t id)
	{
		// The random make takes any gems (FUN_0082637e skips the weapon check in mode 2).
		bool weapon = mode == 2 || (recipe && maker::is_weapon(recipe->item_id));
		std::string refusal = maker::gem_refusal(gems, gem_slots(), id, weapon);

		if (!refusal.empty())
		{
			notice(refusal);

			return;
		}

		gems.push_back(id);
		Sound(Sound::Name::DRAGEND).play();
	}

	void UIMaker::drop_stimulator(int32_t id)
	{
		if (mode != 2 && (mode != 1 || !recipe))
			return;

		// The random make takes any stimulator (FUN_008265a1 skips the check in mode 2).
		std::string refusal = maker::stimulator_refusal(stimulated, id, mode == 2 ? id : recipe->catalyst);

		if (!refusal.empty())
		{
			notice(refusal);

			return;
		}

		stimulated = true;
		stimulator = id;
		Sound(Sound::Name::DRAGEND).play();
	}

	int32_t UIMaker::current_fee() const
	{
		if (mode == 1 && recipe)
			return maker::fee(recipe->meso, stimulated, gems);

		if (mode == 4 && !slots.empty() && slots[0].item_id)
		{
			const maker::Recipe *made = recipe_of(slots[0].item_id, recipes());
			auto equip = Stage::get().get_player().get_inventory().get_equip(InventoryType::Id::EQUIP, equip_slot);

			if (made && equip)
			{
				// The equip's stats against its own, in FUN_005ca1b8's order.
				static const EquipStat::Id GRADED[] = {
					EquipStat::Id::STR, EquipStat::Id::DEX, EquipStat::Id::INT, EquipStat::Id::LUK,
					EquipStat::Id::HP, EquipStat::Id::MP, EquipStat::Id::WATK, EquipStat::Id::MAGIC,
					EquipStat::Id::WDEF, EquipStat::Id::MDEF, EquipStat::Id::ACC, EquipStat::Id::AVOID,
					EquipStat::Id::SPEED, EquipStat::Id::JUMP
				};

				const EquipData &data = EquipData::get(slots[0].item_id);
				std::vector<int32_t> stats;
				std::vector<int32_t> own;

				for (EquipStat::Id stat : GRADED)
				{
					stats.push_back(static_cast<int16_t>(equip->get_stat(stat)));
					own.push_back(data.get_defstat(stat));
				}

				return maker::disassembly_fee(made->meso, maker::stat_grade(stats, own));
			}
		}

		return 0;
	}

	bool UIMaker::has_room(bool tell) const
	{
		// The random make checks neither room nor mesos (FUN_00826fb3).
		if (mode == 2)
			return true;

		const Inventory &inventory = Stage::get().get_player().get_inventory();
		auto free = [&inventory](InventoryType::Id tab)
		{
			int32_t count = 0;

			for (int16_t s = 1; s <= inventory.get_slotmax(tab); s++)
				if (inventory.get_item_id(tab, s) == 0)
					count++;

			return count;
		};

		InventoryType::Id tab = InventoryType::Id::ETC;
		int32_t needed = 1;

		if (mode == 1 && recipe)
			tab = InventoryType::by_item_id(recipe->item_id);

		if (mode == 4 && !slots.empty())
			if (const maker::Recipe *made = recipe_of(slots[0].item_id, recipes()))
				needed = static_cast<int32_t>(made->ingredients.size());

		if (free(tab) >= needed)
			return true;

		if (tell)
			notice(maker::no_room_text(tab));

		return false;
	}

	int32_t UIMaker::gem_slots() const
	{
		if (mode == 2)
			return 4;

		return mode == 1 && recipe ? std::min(recipe->gem_slots, 4) : 0;
	}

	UIElement::Type UIMaker::get_type() const
	{
		return TYPE;
	}

	void UIMaker::answer(int32_t result, int32_t made, const std::string &name, bool disassembled)
	{
		if (state != State::WAITING)
			return;

		state = State::IDLE;
		buttons[Buttons::CANCEL]->set_state(Button::State::NORMAL);
		category.enable(true);
		item.enable(category.selected() != 0);

		std::string text = disassembled ? maker::disassembled_text(name) : maker::result_text(result, made, name);

		if (!text.empty())
			notice(text);

		// A disassembly empties its slot (FUN_008248dd); gems and a stimulator stay in until they run out.
		if (disassembled && mode == 4 && !slots.empty())
			slots[0].item_id = 0;

		// The selection stays; what was used up is counted again.
		recount();
	}

	Button::State UIMaker::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::START:
				start();

				return can_start() ? Button::State::NORMAL : Button::State::DISABLED;
			case Buttons::CANCEL:
			case Buttons::CLOSE:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIMaker::pick_category()
	{
		int32_t key = category.data();
		std::vector<DropList::Choice> choices;

		mode = 0;
		recipe = nullptr;
		slots.clear();
		gems.clear();
		stimulated = false;
		stimulator = 0;

		// FUN_0082384f.
		if (category.selected() != 0)
		{
			choices.push_back({ "Item", 0 });

			switch (key)
			{
				case maker::DISASSEMBLE:
				case maker::CRYSTAL:
					choices.push_back({ "Monster Crystal", key });
					break;
				case maker::RANDOM:
					choices.push_back({ "???", key });
					break;
				default:
					for (const maker::Recipe *r : maker::under(key, listed))
						choices.push_back({ ItemData::get(r->item_id).get_name(), r->item_id });
					break;
			}
		}

		item.set(choices);
		item.enable(!choices.empty());
		recount();
	}

	void UIMaker::pick_item()
	{
		int32_t data = item.data();

		mode = 0;
		recipe = nullptr;
		slots.clear();
		gems.clear();
		stimulated = false;
		stimulator = 0;

		// FUN_00823a13.
		if (item.selected() != 0)
		{
			switch (data)
			{
				case maker::CRYSTAL:
					show_crystal();
					break;
				case maker::DISASSEMBLE:
					// The disassembly takes an equip dropped in.
					mode = 4;
					slots.resize(1);
					slots[0].needed = 1;
					break;
				case maker::RANDOM:
					mode = 2;
					slots.resize(6);
					break;
				default:
					show_recipe(data);
					break;
			}
		}

		recount();
	}

	void UIMaker::show_recipe(int32_t item_id)
	{
		for (const maker::Recipe *r : listed)
		{
			if (r->item_id != item_id)
				continue;

			mode = 1;
			recipe = r;

			for (auto [id, needed] : r->ingredients)
			{
				if (slots.size() == 6)
					break;

				slots.push_back({ id, 0, needed });
			}

			return;
		}
	}

	void UIMaker::show_crystal()
	{
		mode = 3;
		slots.resize(1);
		slots[0].needed = maker::CRYSTAL_DROPS;
	}

	void UIMaker::recount()
	{
		const Inventory &inventory = Stage::get().get_player().get_inventory();

		// The equipment to take apart must stay where it was.
		if (mode == 4 && !slots.empty() && slots[0].item_id &&
			inventory.get_item_id(InventoryType::Id::EQUIP, equip_slot) != slots[0].item_id)
			slots[0].item_id = 0;

		// Gems and the stimulator come out once none are left (FUN_00824a90).
		gems.erase(std::remove_if(gems.begin(), gems.end(), [&inventory](int32_t gem)
		{
			return inventory.get_total_item_count(gem) == 0;
		}), gems.end());

		if (stimulated && inventory.get_total_item_count(stimulator) == 0)
		{
			stimulated = false;
			stimulator = 0;
		}

		// The random make keeps what the player said, but no more than they have.
		for (Slot &slot : slots)
			if (mode == 2)
				slot.owned = slot.item_id ? std::min<int32_t>(slot.owned, inventory.get_total_item_count(slot.item_id)) : 0;
			else
				slot.owned = slot.item_id ? inventory.get_total_item_count(slot.item_id) : 0;

		if (state == State::IDLE || state == State::READY)
		{
			state = mode ? State::READY : State::IDLE;
			Button &start = *buttons[Buttons::START];
			bool on = can_start();

			// Only a change, so a hovered or pressed Start keeps its look.
			if (!on)
				start.set_state(Button::State::DISABLED);
			else if (start.get_state() == Button::State::DISABLED)
				start.set_state(Button::State::NORMAL);
		}
	}

	bool UIMaker::can_start() const
	{
		switch (mode)
		{
			case 1:
			{
				if (!recipe || !maker::listed(*recipe, facts))
					return false;

				for (const Slot &slot : slots)
					if (slot.owned < slot.needed)
						return false;

				return true;
			}
			case 3:
			case 4:
				return !slots.empty() && slots[0].item_id && slots[0].owned >= slots[0].needed;
			case 2:
				return std::any_of(slots.begin(), slots.end(), [](const Slot &slot) { return slot.item_id != 0; });
			default:
				return false;
		}
	}

	void UIMaker::start()
	{
		if (state != State::READY || !can_start())
			return;

		std::string name = mode == 1 ? ItemData::get(recipe->item_id).get_name() : "";

		UI::get().emplace<UIYesNo>(maker::confirm_text(mode, name), [](bool yes)
		{
			auto window = UI::get().get_element<UIMaker>();

			if (yes && window && window->is_active())
				window->confirm();
		});
	}

	void UIMaker::confirm()
	{
		// Things may have moved while it asked (FUN_00826f25), and the fee is checked now (FUN_00826fb3).
		recount();

		if (state != State::READY || !can_start() || !has_room(true))
			return;

		// A disassembly short of mesos does nothing.
		if (Stage::get().get_player().get_inventory().get_meso() < current_fee())
		{
			if (mode != 4)
				notice("You do not have enough mesos.");

			return;
		}

		state = State::FILLING;
		elapsed = 0;
		category.enable(false);
		item.enable(false);
		buttons[Buttons::START]->set_state(Button::State::DISABLED);
		working.play_loop();
	}

	void UIMaker::send()
	{
		working.stop();

		// What was checked at Start is checked again (FUN_00827096).
		state = State::READY;
		recount();

		// No recipe is hidden, so the random make never finds what was put in (FUN_00822ce1).
		if (mode == 2 || !can_start() || !has_room(false) ||
			Stage::get().get_player().get_inventory().get_meso() < current_fee())
		{
			notice("Production failed.");
			state = State::IDLE;
			buttons[Buttons::CANCEL]->set_state(Button::State::NORMAL);
			category.enable(true);
			item.enable(category.selected() != 0);
			recount();

			return;
		}

		state = State::WAITING;
		buttons[Buttons::START]->set_state(Button::State::DISABLED);

		if (mode == 1)
			ItemMakePacket(recipe->item_id, stimulated, gems).dispatch();
		else if (mode == 3)
			ItemMakeCrystalPacket(slots[0].item_id).dispatch();
		else if (mode == 4)
			ItemDisassemblePacket(slots[0].item_id, equip_slot).dispatch();
	}

	void UIMaker::close()
	{
		working.stop();
		clear_tooltip();
		deactivate();
	}

	void UIMaker::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::MAKER);
	}

	void UIMaker::show_tooltip(Point<int16_t> cursor)
	{
		// FUN_00826ace: the result's item, an ingredient, or where the drops go.
		if (mode == 1 && recipe && RESULT.contains(cursor))
		{
			UI::get().show_item(Tooltip::Parent::MAKER, recipe->item_id);

			return;
		}

		int32_t i = slot_at(cursor);

		if (i >= 0 && slots[i].item_id)
		{
			UI::get().show_item(Tooltip::Parent::MAKER, slots[i].item_id);

			return;
		}

		for (size_t k = 0; k < gems.size(); k++)
		{
			if (square(gem_position(static_cast<int32_t>(k))).contains(cursor))
			{
				UI::get().show_item(Tooltip::Parent::MAKER, gems[k]);

				return;
			}
		}

		if (stimulated && mode == 2 && square(STIMULATOR).contains(cursor))
		{
			UI::get().show_item(Tooltip::Parent::MAKER, stimulator);

			return;
		}

		if (mode == 1 && recipe && recipe->catalyst && square(STIMULATOR).contains(cursor))
		{
			UI::get().show_item(Tooltip::Parent::MAKER, recipe->catalyst);

			return;
		}

		if ((mode == 3 || mode == 4) && INGREDIENTS.contains(cursor))
		{
			UI::get().show_text(Tooltip::Parent::MAKER, mode == 3 ? CRYSTAL_HINT : DISASSEMBLY_HINT);

			return;
		}

		clear_tooltip();
	}

	int32_t UIMaker::slot_at(Point<int16_t> cursor) const
	{
		for (size_t i = 0; i < slots.size(); i++)
		{
			Point<int16_t> at = slot_position(i);

			if (square(at).contains(cursor))
				return static_cast<int32_t>(i);
		}

		return -1;
	}
}
