#pragma once

#include "../UIElement.h"
#include "../Components/FadeNotice.h"

#include "../../Character/Inventory/Inventory.h"
#include "../../Character/QuestCompletion.h"
#include "../../Character/QuestLog.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// v83's red "Quest Complete!" notice (UIWindow.img/FadeYesNo/backgrnd4)
	// above the status bar, shown when a quest's last objective is met. O
	// opens the quest in the quest log and X dismisses it. Each notice stays
	// until then, or until the quest is no longer in progress; newer notices
	// stack above older ones, invitations included.
	class UIQuestComplete : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::QUESTCOMPLETE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIQuestComplete(QuestLog& quests, const Inventory& inventory);

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Notice i's buttons are OK + 2i and CANCEL + 2i.
		enum Buttons : uint16_t { OK, CANCEL };

		struct Notice
		{
			int16_t id;
			Text name;
			FadeNotice life;
		};

		void rebuild();
		// From the bottom of the stack to `notice`.
		Point<int16_t> offset(const Notice& notice) const;

		QuestLog& quests;
		const Inventory& inventory;
		QuestCompletion completion;
		Texture background, icon;
		Text heading;
		// Oldest first; the newest is drawn last, over the others.
		std::vector<Notice> notices;
		// How high the buttons sit, to move them when the stack shifts.
		int16_t buttons_raised = -1;

		static constexpr int16_t WIDTH = 155, HEIGHT = 44;
	};
}
