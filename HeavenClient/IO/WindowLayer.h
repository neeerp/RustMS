#pragma once

#include <algorithm>
#include <list>

namespace ms
{
	namespace window_layer
	{
		// v83 creates the fade cards (FUN_0051f9ad) and every dialog (CDialog,
		// FUN_004edaef) at z 10, above the status bar and the ordinary windows.
		// Clicking a window raises it only among the windows of its own z, so
		// a card stays over the status bar whatever was clicked last, while a
		// dialog opened after a card still covers it (reports/182-3, #608).
		//
		// Moves type to the top of its layer in a back-to-front draw order.
		template <typename Type, typename IsTop>
		void raise(std::list<Type> &order, Type type, IsTop is_top)
		{
			order.remove(type);

			if (is_top(type))
				order.push_back(type);
			else
				order.insert(std::find_if(order.begin(), order.end(), is_top), type);
		}
	}
}
