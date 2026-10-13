#pragma once

#include "../UIElement.h"

#include "../../Character/QuestLog.h"
#include "../../Character/QuestObjectives.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <vector>

namespace ms
{
	class Inventory;

	// The v83 Quest Helper (UIWindow.img/QuestAlarm): tracked quests and what
	// each still needs, flush with the top-right corner of the screen.
	class UIQuestHelper : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::QUESTHELPER;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIQuestHelper(QuestLog& quests, const Inventory& inventory);

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Each tracked quest's untrack button is UNTRACK plus its place.
		enum Buttons : uint16_t { AUTO, MINIMIZE, MAXIMIZE, CLOSE, UNTRACK };

		struct Requirement
		{
			// The mob or item, then how many there are and how many it takes.
			Text name, have, need;
		};

		struct Entry
		{
			int16_t id;
			Text name;
			std::vector<Requirement> requirements;
		};

		void rebuild();
		int16_t height() const;

		QuestLog& quests;
		const Inventory& inventory;
		Texture top, top_minimized, center, bottom, icon;
		Text title;
		std::vector<Entry> entries;
		// What the entries were built from, to rebuild only on changes.
		std::vector<std::pair<int16_t, std::vector<QuestObjective>>> shown;
		bool minimized = false;

		static constexpr int16_t WIDTH = 223, ROW = 18, TOP = 25, TOP_MINIMIZED = 20, BOTTOM = 5;
	};
}
